use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
}

fn get_jwt_secret() -> Vec<u8> {
    env::var("JWT_SECRET")
        .expect("JWT_SECRET environment variable must be set with a strong secret key (min 32 characters)")
        .into_bytes()
}

pub fn create_jwt(id: &str, role: &str) -> anyhow::Result<String> {
    let expiration = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 3600;

    let claims = Claims {
        sub: id.to_owned(),
        role: role.to_owned(),
        exp: expiration as usize,
    };

    let key = get_jwt_secret();
    encode(&Header::default(), &claims, &EncodingKey::from_secret(&key))
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn verify_jwt(token: &str) -> anyhow::Result<Claims> {
    let key = get_jwt_secret();
    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(&key), &validation)
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn set_jwt_secret() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret_key_must_be_long_enough_32_chars");
        }
    }

    #[test]
    #[serial]
    fn test_create_and_verify_jwt() {
        set_jwt_secret();
        let token = create_jwt("user-123", "admin").unwrap();
        let claims = verify_jwt(&token).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.role, "admin");
    }

    #[test]
    #[serial]
    fn test_jwt_expiration_set() {
        set_jwt_secret();
        let token = create_jwt("user-456", "operator").unwrap();
        let claims = verify_jwt(&token).unwrap();
        assert!(claims.exp > 0);
        // Expiration should be roughly 1 hour from now
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(claims.exp > now);
        assert!(claims.exp <= now + 3601);
    }

    #[test]
    #[serial]
    fn test_verify_invalid_token() {
        set_jwt_secret();
        let result = verify_jwt("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_verify_empty_token() {
        set_jwt_secret();
        let result = verify_jwt("");
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: "test-id".to_string(),
            role: "admin".to_string(),
            exp: 9999999999,
        };
        let json = serde_json::to_string(&claims).unwrap();
        let parsed: Claims = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sub, "test-id");
        assert_eq!(parsed.role, "admin");
        assert_eq!(parsed.exp, 9999999999);
    }

    #[test]
    #[serial]
    fn test_jwt_different_roles() {
        set_jwt_secret();
        for role in &["admin", "operator", "viewer"] {
            let token = create_jwt("user-1", role).unwrap();
            let claims = verify_jwt(&token).unwrap();
            assert_eq!(claims.role, *role);
        }
    }
}
