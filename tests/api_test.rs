use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use bookmarks_api::{
    db::bookmark::BookmarkRepo,
    error::AppError,
    handlers::bookmark::AppState,
    models::bookmark::{Bookmark, CreateBookmark, UpdateBookmark},
    routes::bookmark::bookmark_routes,
};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_app_setup(pool: PgPool) -> Router {
    let state = AppState {
        bookmark_repo: BookmarkRepo::new(pool),
    };
    bookmark_routes().with_state(state)
}

fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[sqlx::test]
async fn create_persists_and_reads_back(pool: PgPool) {
    let repo = BookmarkRepo::new(pool);
    let input = CreateBookmark {
        title: "Rust".to_string(),
        url: "https://rust-lang.org".to_string(),
        description: Some("A systems language".to_string()),
    };
    let created = repo.create(input).await.unwrap();
    let fetched = repo.get_by_id(created.id).await.unwrap();

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.title, "Rust");
    assert_eq!(fetched.url, "https://rust-lang.org");
    assert_eq!(fetched.description, Some("A systems language".to_string()));
}

#[sqlx::test]
async fn get_by_id_missing_returns_not_found(pool: PgPool) {
    let repo = BookmarkRepo::new(pool);

    let result = repo.get_by_id(Uuid::new_v4()).await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[sqlx::test]
async fn delete_removes_bookmark(pool: PgPool) {
    let repo = BookmarkRepo::new(pool);

    let created = repo
        .create(CreateBookmark {
            title: "Temp".to_string(),
            url: "https://example.com".to_string(),
            description: None,
        })
        .await
        .unwrap();

    assert!(repo.delete(created.id).await.unwrap());

    assert!(matches!(
        repo.get_by_id(created.id).await,
        Err(AppError::NotFound(_))
    ));

    assert!(!repo.delete(created.id).await.unwrap());
}

#[sqlx::test]
async fn list_returns_all_bookmarks(pool: PgPool) {
    let repo = BookmarkRepo::new(pool);

    for title in ["One", "Two", "Three"] {
        repo.create(CreateBookmark {
            title: title.to_string(),
            url: format!("https://example.com/{title}"),
            description: None,
        })
        .await
        .unwrap();
    }

    let all = repo.get_all().await.unwrap();

    assert_eq!(all.len(), 3);
    let titles: Vec<&str> = all.iter().map(|b| b.title.as_str()).collect();

    assert!(titles.contains(&"One"));
    assert!(titles.contains(&"Two"));
    assert!(titles.contains(&"Three"));
}

#[sqlx::test]
async fn update_changes_fields(pool: PgPool) {
    let repo = BookmarkRepo::new(pool);

    let created = repo
        .create(CreateBookmark {
            title: "Old title".to_string(),
            url: "https://old.example.com".to_string(),
            description: Some("old".to_string()),
        })
        .await
        .unwrap();

    let updated = repo
        .update(
            created.id,
            UpdateBookmark {
                title: Some("New title".to_string()),
                url: None,
                description: Some("new".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.title, "New title");
    assert_eq!(updated.description, Some("new".to_string()));
    assert_eq!(updated.url, "https://old.example.com");
}

#[sqlx::test]
async fn http_create_returns_ok(pool: PgPool) {
    let app = test_app_setup(pool);

    let req = json_request(
        "POST",
        "/bookmarks",
        serde_json::json!({
            "title": "Via HTTP",
            "url": "https://http.example.com",
            "description": "made over http"
        }),
    );

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[sqlx::test]
async fn http_get_missing_returns_not_found(pool: PgPool) {
    let app = test_app_setup(pool);

    let req = Request::builder()
        .method("GET")
        .uri(format!("/bookmarks/{}", Uuid::new_v4()))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn http_delete_returns_no_content(pool: PgPool) {
    // Seed straight through the repo. PgPool is Clone (Arc inside), so we can
    // hand one clone to the repo and the original to the app.
    let repo = BookmarkRepo::new(pool.clone());
    let created = repo
        .create(CreateBookmark {
            title: "To delete".to_string(),
            url: "https://delete.example.com".to_string(),
            description: None,
        })
        .await
        .unwrap();

    let app = test_app_setup(pool);

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/bookmarks/{}", created.id))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn http_list_returns_all(pool: PgPool) {
    let repo = BookmarkRepo::new(pool.clone());
    for title in ["A", "B"] {
        repo.create(CreateBookmark {
            title: title.to_string(),
            url: format!("https://example.com/{title}"),
            description: None,
        })
        .await
        .unwrap();
    }

    let app = test_app_setup(pool);

    let req = Request::builder()
        .method("GET")
        .uri("/bookmarks")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let bookmarks: Vec<Bookmark> = serde_json::from_slice(&body).unwrap();
    assert_eq!(bookmarks.len(), 2);
}
