use actix_web::HttpRequest;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{decode, errors::Error as JwtError, DecodingKey, TokenData, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[allow(dead_code)]
pub const SECRET: &[u8] = b"dev_secret_key_change_me";

#[allow(dead_code)]
pub fn validate_jwt_from_request(req: &HttpRequest) -> Result<TokenData<Claims>, JwtError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").to_string());

    match auth_header {
        Some(token) => decode::<Claims>(
            &token,
            &DecodingKey::from_secret(SECRET),
            &Validation::default(),
        ),
        None => Err(JwtError::from(ErrorKind::InvalidToken)), // ✅ Fixed
    }
}
