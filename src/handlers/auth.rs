use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::{SaltString, rand_core::OsRng}};
use axum::{Json, extract::State};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::{AppError, AppResult}, handlers::bookmark::AppState, models::bookmark::{LoginRequest, LoginResponse, RegisterRequest, User}};

fn hash_secret(plain: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| AppError::InternalError(anyhow::anyhow!(e.to_string())))?
        .to_string();
    Ok(hash)
}

fn verify_secret(plain: &str, stored_hash: &str) -> AppResult<bool>{
    let parsed = PasswordHash::new(stored_hash)
        .map_err(|e| AppError::InternalError(anyhow::anyhow!(e.to_string())))?;
    Ok(Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    pub iat: usize,
    pub exp: usize,
}

fn create_token(user_id: Uuid, secret: &str) -> AppResult<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(24)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalError(anyhow::anyhow!(e.to_string())))
}

pub async fn register_user(
    State(state): State<AppState>,
    Json(input): Json<RegisterRequest>,
) -> AppResult<Json<User>> {
    let password_hash = hash_secret(&input.password)?;
    let user = state.user_repo.create_user(&input.email, &password_hash).await?;
    Ok(Json(user))
}

pub async fn login_user(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let user = match state.user_repo.find_by_email(&input.email).await {
        Ok(u) => u,
        Err(AppError::NotFound(_)) => return Err(AppError::Unauthorized),
        Err(e) => return Err(e),
    };
    if !verify_secret(&input.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }
    let token = create_token(user.id, &state.jwt_secret)?;
    Ok(Json(LoginResponse { token }))
}