use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::bookmark::{Bookmark, CreateBookmark, UpdateBookmark},
};

#[derive(Clone)]
pub struct BookmarkRepo {
    pool: PgPool,
}

impl BookmarkRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_all(&self) -> AppResult<Vec<Bookmark>> {
        let bookmarks =
            sqlx::query_as!(Bookmark, "SELECT * FROM bookmarks ORDER BY created_at DESC",)
                .fetch_all(&self.pool)
                .await?;

        Ok(bookmarks)
    }

    pub async fn get_by_id(&self, id: Uuid) -> AppResult<Bookmark> {
        let bookmark = sqlx::query_as!(Bookmark, "SELECT * FROM bookmarks WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Bookmark {id} not found")))?;

        Ok(bookmark)
    }

    pub async fn create(&self, input: CreateBookmark) -> AppResult<Bookmark> {
        let now = Utc::now();
        let id = Uuid::new_v4();

        let bookmark = sqlx::query_as!(
            Bookmark,
            "
            INSERT INTO bookmarks (id, title, url, description, created_at)
            VALUES($1, $2, $3, $4, $5)
            RETURNING *
            ",
            id,
            input.title,
            input.url,
            input.description,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(bookmark)
    }

    pub async fn update(&self, id: Uuid, input: UpdateBookmark) -> AppResult<Bookmark> {
        let bookmark = sqlx::query_as!(
            Bookmark,
            "
            UPDATE bookmarks
            SET
                title = COALESCE($2, title),
                url = COALESCE($3, url),
                description = COALESCE($4, description)
            WHERE id = $1
            RETURNING *
            ",
            id,
            input.title,
            input.url,
            input.description
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Bookmark {id} not found")))?;

        Ok(bookmark)
    }

    pub async fn delete(&self, id: Uuid) -> AppResult<bool> {
        let result = sqlx::query!("DELETE FROM bookmarks WHERE id = $1", id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
