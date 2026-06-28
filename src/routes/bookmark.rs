use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use crate::handlers::{auth::{login_user, register_user}, bookmark::{
    AppState, create_bookmark, delete_bookmark, get_bookmark, list_all_bookmarks, update_bookmark,
}};

pub fn bookmark_routes() -> Router<AppState> {
    Router::new()
        .route("/bookmarks", get(list_all_bookmarks))
        .route("/bookmarks", post(create_bookmark))
        .route("/bookmarks/{id}", get(get_bookmark))
        .route("/bookmarks/{id}", patch(update_bookmark))
        .route("/bookmarks/{id}", delete(delete_bookmark))
        .route("/register", post(register_user))
        .route("/login", post(login_user))
}

// pub fn bookmark_routes() -> Router<AppState> {
//     Router::new()
//         .route("/bookmarks", get(list_all_bookmarks).post(create_bookmark))
//         .route(
//             "/bookmarks/{id}",
//             get(get_bookmark)
//                 .patch(update_bookmark)
//                 .delete(delete_bookmark),
//         )
// }
