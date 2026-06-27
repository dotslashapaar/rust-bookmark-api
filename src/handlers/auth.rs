use argon2::{Argon2, PasswordHasher, password_hash::{SaltString, rand_core::OsRng}};
use axum::{Json, extract::State};

use crate::{error::{AppError, AppResult}, handlers::bookmark::AppState, models::bookmark::{RegisterRequest, User}};

fn hash_secret(plain: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| AppError::InternalError(anyhow::anyhow!(e.to_string())))?
        .to_string();
    Ok(hash)
}

pub async fn register_user(
    State(state): State<AppState>,
    Json(input): Json<RegisterRequest>,
) -> AppResult<Json<User>> {
    let password_hash = hash_secret(&input.password)?;
    let user = state.user_repo.create_user(&input.email, &password_hash).await?;
    Ok(Json(user))
}
