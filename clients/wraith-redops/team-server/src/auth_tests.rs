use super::RpcPath;
use tonic::{Code, Request, Status};

fn wrap_auth_interceptor(req: Request<()>) -> Result<Request<()>, Status> {
    super::auth_interceptor(req)
}

#[test]
fn test_auth_interceptor_rejects_missing_header() {
    let req = Request::new(());

    let result = wrap_auth_interceptor(req);

    match result {
        Err(status) => {
            assert_eq!(status.code(), Code::Unauthenticated);
            assert_eq!(status.message(), "Missing authorization header");
        }
        Ok(_) => panic!("Should have rejected missing header"),
    }
}

#[test]
fn test_auth_interceptor_whitelists_authenticate() {
    let mut req = Request::new(());
    req.extensions_mut().insert(RpcPath(
        "/wraith.redops.OperatorService/Authenticate".to_string(),
    ));

    let result = wrap_auth_interceptor(req);

    assert!(
        result.is_ok(),
        "Authenticate should be whitelisted even without header"
    );
}

#[test]
fn test_auth_interceptor_invalid_scheme() {
    let mut req = Request::new(());
    req.metadata_mut()
        .insert("authorization", "Basic dXNlcjpwYXNz".parse().unwrap());

    let result = wrap_auth_interceptor(req);

    match result {
        Err(status) => {
            assert_eq!(status.code(), Code::Unauthenticated);
            assert_eq!(status.message(), "Invalid auth scheme");
        }
        Ok(_) => panic!("Should have failed with invalid auth scheme"),
    }
}

#[test]
fn test_auth_interceptor_accepts_valid_token() {
    unsafe {
        std::env::set_var("JWT_SECRET", "test_secret_key_must_be_long_enough_32_chars");
    }

    let user_id = "550e8400-e29b-41d4-a716-446655440000";
    let token = crate::utils::create_jwt(user_id, "admin").unwrap();

    let mut req = Request::new(());
    req.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token).parse().unwrap(),
    );

    let result = wrap_auth_interceptor(req);

    assert!(result.is_ok());
    let claims = result
        .unwrap()
        .extensions()
        .get::<crate::utils::Claims>()
        .cloned()
        .unwrap();
    assert_eq!(claims.sub, user_id);
}
