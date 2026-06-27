use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    db::{bookmark::BookmarkRepo, user::UserRepo}, error::{AppError, AppResult}, models::bookmark::{Bookmark, CreateBookmark, UpdateBookmark},
};

#[derive(Clone)]
pub struct AppState {
    pub bookmark_repo: BookmarkRepo,
    pub user_repo: UserRepo
}

pub async fn list_all_bookmarks(State(state): State<AppState>) -> AppResult<Json<Vec<Bookmark>>> {
    let bookmarks = state.bookmark_repo.get_all().await?;
    Ok(Json(bookmarks))
}

pub async fn get_bookmark(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Bookmark>> {
    let bookmarks = state.bookmark_repo.get_by_id(id).await?;
    Ok(Json(bookmarks))
}

pub async fn create_bookmark(
    State(state): State<AppState>,
    Json(input): Json<CreateBookmark>,
) -> AppResult<Json<Bookmark>> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("Title cannot be empty".to_string()));
    }

    let bookmark = state.bookmark_repo.create(input).await?;

    Ok(Json(bookmark))
}

pub async fn update_bookmark(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateBookmark>,
) -> AppResult<Json<Bookmark>> {
    let bookmark = state.bookmark_repo.update(id, input).await?;

    Ok(Json(bookmark))
}

pub async fn delete_bookmark(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let deleted = state.bookmark_repo.delete(id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!(
            "Bookmark with id {} not found",
            id
        )))
    }
}
