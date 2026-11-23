use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use serde_json::json;
use uuid::Uuid;

use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::features::auth::middleware::AuthUser;
use crate::features::auth::models::{
    AuthStatusResponse, AuthSuccessResponse, LoginRequest, SetupRequest, User, UserResponse,
};
use crate::features::auth::service::{create_jwt_token, hash_password, verify_jwt_token, verify_password};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/setup", post(setup_admin))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/status", get(get_status))
        .route("/me", get(get_current_user))
}

async fn setup_admin(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<SetupRequest>,
) -> Result<impl IntoResponse, AppError> {
    let username = payload.username.trim();
    if username.len() < 3 {
        return Err(AppError::BadRequest("Username must be at least 3 characters long".into()));
    }
    if payload.password.len() < 8 {
        return Err(AppError::BadRequest("Password must be at least 8 characters long".into()));
    }

    // 1. Ensure setup is only permitted if no users exist
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;

    if count.0 > 0 {
        return Err(AppError::BadRequest("Initial setup has already been completed".into()));
    }

    // 2. Hash password and insert first admin user
    let password_hash = hash_password(&payload.password)?;
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password_hash, role) VALUES ($1, $2, 'admin') RETURNING *"
    )
    .bind(username)
    .bind(&password_hash)
    .fetch_one(&state.db)
    .await?;

    // 3. Issue JWT Token
    let token = create_jwt_token(&user, &state.config.jwt_secret, state.config.jwt_expiration_hours)?;

    // 4. Set HttpOnly session cookie
    let cookie = Cookie::build((state.config.cookie_name.clone(), token.clone()))
        .path("/")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax)
        .build();

    let user_res = UserResponse::from(user);
    let body = AuthSuccessResponse {
        message: "Initial administrator user created successfully".into(),
        user: user_res,
        token,
    };

    Ok((StatusCode::CREATED, jar.add(cookie), Json(body)))
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let username = payload.username.trim();

    // 1. Look up user by username
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid username or password".into()))?;

    // 2. Verify password hash
    let valid = verify_password(&payload.password, &user.password_hash)?;
    if !valid {
        return Err(AppError::Unauthorized("Invalid username or password".into()));
    }

    // 3. Issue JWT Token
    let token = create_jwt_token(&user, &state.config.jwt_secret, state.config.jwt_expiration_hours)?;

    // 4. Set HttpOnly session cookie
    let cookie = Cookie::build((state.config.cookie_name.clone(), token.clone()))
        .path("/")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax)
        .build();

    let user_res = UserResponse::from(user);
    let body = AuthSuccessResponse {
        message: "Authenticated successfully".into(),
        user: user_res,
        token,
    };

    Ok((StatusCode::OK, jar.add(cookie), Json(body)))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let mut removal_cookie = Cookie::build((state.config.cookie_name.clone(), ""))
        .path("/")
        .build();
    removal_cookie.make_removal();

    Ok((
        StatusCode::OK,
        jar.add(removal_cookie),
        Json(json!({ "message": "Logged out successfully" })),
    ))
}

async fn get_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<Json<AuthStatusResponse>, AppError> {
    // 1. Check if setup is required
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

    let setup_required = count.0 == 0;

    // 2. Check if currently authenticated
    let token_from_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|val| val.to_str().ok())
        .and_then(|val| val.strip_prefix("Bearer "));

    let token_from_cookie = jar.get(&state.config.cookie_name).map(|c| c.value());

    let authenticated_user = match token_from_header.or(token_from_cookie) {
        Some(token) => match verify_jwt_token(token, &state.config.jwt_secret) {
            Ok(claims) => {
                let user_id = Uuid::parse_str(&claims.sub).ok();
                user_id.map(|id| UserResponse {
                    id,
                    username: claims.username,
                    role: claims.role,
                })
            }
            Err(_) => None,
        },
        None => None,
    };

    let is_auth = authenticated_user.is_some();

    Ok(Json(AuthStatusResponse {
        setup_required,
        authenticated: is_auth,
        user: authenticated_user,
    }))
}

async fn get_current_user(AuthUser(user): AuthUser) -> Json<UserResponse> {
    Json(user)
}
