use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
}

fn get_jwt_secret() -> Vec<u8> {
    env::var("JWT_SECRET").unwrap_or_else(|_| "secret_key_wraith_redops".to_string()).into_bytes()
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