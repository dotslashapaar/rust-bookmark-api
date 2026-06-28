use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::bookmark::User,
};

#[derive(Clone)]
pub struct UserRepo {
    pool: PgPool,
}

impl UserRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_user(&self, email: &str, password_hash: &str) -> AppResult<User> {
        let id = Uuid::new_v4();
        let user = sqlx::query_as!(
            User,
            "INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3) RETURNING *",
            id,
            email,
            password_hash
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::Conflict("Email already in use".to_string())
            }
            other => AppError::DbError(other),
        })?;

        Ok(user)
    }

    pub async fn find_by_email(&self, email: &str) -> AppResult<User> {
        let user = sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", email)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {email} not found")))?;
        Ok(user)
    }
}
