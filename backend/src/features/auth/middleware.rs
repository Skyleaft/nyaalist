use axum::{
    async_trait,
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts},
    middleware::Next,
    response::Response,
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

        // 3. Try to read token from Query string (?token=...)
        let token_from_query = parts.uri.query().and_then(|q| {
            q.split('&').find_map(|pair| {
                let mut split = pair.splitn(2, '=');
                let key = split.next()?;
                let val = split.next()?;
                if key == "token" {
                    urlencoding::decode(val).ok().map(|s| s.into_owned())
                } else {
                    None
                }
            })
        });

        let token_borrowed = token_from_header.or(token_from_cookie);
        let token_owned = token_from_query;

        let token_str = match (token_borrowed, &token_owned) {
            (Some(t), _) => t,
            (None, Some(t)) => t.as_str(),
            (None, None) => return Err(AppError::Unauthorized("Authentication credentials missing".into())),
        };

        // 4. Verify JWT
        let claims = verify_jwt_token(token_str, &state.config.jwt_secret)?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user identity format".into()))?;

        Ok(AuthUser(UserResponse {
            id: user_id,
            username: claims.username,
            role: claims.role,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct AdminUser(pub UserResponse);

#[async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;
        if user.role != "admin" {
            return Err(AppError::Forbidden("Administrator privileges required".into()));
        }
        Ok(AdminUser(user))
    }
}

#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<UserResponse>);

#[async_trait]
impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(AuthUser(user)) => Ok(OptionalAuthUser(Some(user))),
            Err(_) => Ok(OptionalAuthUser(None)),
        }
    }
}

/// Route-level middleware to enforce authentication
pub async fn require_auth(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let (mut parts, body) = req.into_parts();
    let auth_user = AuthUser::from_request_parts(&mut parts, &state).await?;
    parts.extensions.insert(auth_user);
    let req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}

/// Route-level middleware to enforce administrator authorization
pub async fn require_admin(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let (mut parts, body) = req.into_parts();
    let AuthUser(user) = AuthUser::from_request_parts(&mut parts, &state).await?;
    if user.role != "admin" {
        return Err(AppError::Forbidden("Administrator privileges required".into()));
    }
    parts.extensions.insert(AdminUser(user));
    let req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}
