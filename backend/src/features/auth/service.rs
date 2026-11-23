use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, PasswordHash,
};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use crate::core::error::AppError;
use crate::features::auth::models::{Claims, User};

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash format: {}", e)))?;
    let argon2 = Argon2::default();
    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

pub fn create_jwt_token(user: &User, secret: &str, exp_hours: i64) -> Result<String, AppError> {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::hours(exp_hours))
        .expect("Valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: user.role.clone(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Failed to generate JWT token: {}", e)))
}

pub fn verify_jwt_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let validation = Validation::default();
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|token_data| token_data.claims)
    .map_err(|_| AppError::Unauthorized("Invalid or expired session token".into()))
}
