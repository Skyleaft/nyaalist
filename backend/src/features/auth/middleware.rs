use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::features::auth::models::UserResponse;
use crate::features::auth::service::verify_jwt_token;

#[derive(Debug, Clone)]
pub struct AuthUser(pub UserResponse);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        // 1. Try to read token from Authorization: Bearer <token>
        let token_from_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|val| val.to_str().ok())
            .and_then(|val| val.strip_prefix("Bearer "));

        // 2. Try to read token from Cookie if not in header
        let cookie_jar = CookieJar::from_headers(&parts.headers);
        let token_from_cookie = cookie_jar
            .get(&state.config.cookie_name)
            .map(|c| c.value());

        let token = token_from_header
            .or(token_from_cookie)
            .ok_or_else(|| AppError::Unauthorized("Authentication credentials missing".into()))?;

        // 3. Verify JWT
        let claims = verify_jwt_token(token, &state.config.jwt_secret)?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user identity format".into()))?;

        Ok(AuthUser(UserResponse {
            id: user_id,
            username: claims.username,
            role: claims.role,
        }))
    }
}
