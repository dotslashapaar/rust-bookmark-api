use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use bookmarks_api::{
    db::{bookmark::BookmarkRepo, user::UserRepo},
    error::AppError,
    handlers::bookmark::AppState,
    models::bookmark::{Bookmark, CreateBookmark, UpdateBookmark},
    routes::bookmark::bookmark_routes,
};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const JWT_SECRET: &str = "test-secret";

fn test_app(pool: PgPool) -> Router {
    let state = AppState {
        bookmark_repo: BookmarkRepo::new(pool.clone()),
        user_repo: UserRepo::new(pool),
        jwt_secret: JWT_SECRET.to_string(),
    };
    bookmark_routes().with_state(state)
}

fn json_request(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn auth_json(method: &str, uri: &str, body: Value, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn auth_empty(method: &str, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

// Router is Clone (Arc inside) and oneshot consumes self, so we clone per
// request — lets a single app serve a register -> login -> protected flow.
async fn send(app: &Router, req: Request<Body>) -> Response {
    app.clone().oneshot(req).await.unwrap()
}

async fn body_text(res: Response) -> String {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn body_json<T: DeserializeOwned>(res: Response) -> T {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

// Insert a user row directly so repo-level tests have a valid owner_id (the FK
// owner_id REFERENCES users(id) rejects bookmarks for non-existent users).
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    UserRepo::new(pool.clone())
        .create_user(email, "dummy-hash")
        .await
        .unwrap()
        .id
}

// Register then log in over HTTP, returning the bearer token.
async fn register_and_login(app: &Router, email: &str, password: &str) -> String {
    let reg = send(
        app,
        json_request(
            "POST",
            "/register",
            json!({ "email": email, "password": password }),
        ),
    )
    .await;
    assert_eq!(reg.status(), StatusCode::OK, "register should succeed");

    let login = send(
        app,
        json_request(
            "POST",
            "/login",
            json!({ "email": email, "password": password }),
        ),
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK, "login should succeed");

    let v: Value = body_json(login).await;
    v["token"].as_str().unwrap().to_string()
}

// ----- repo-level tests (now owner-scoped) -----

#[sqlx::test]
async fn create_persists_and_reads_back(pool: PgPool) {
    let uid = seed_user(&pool, "create@test.com").await;
    let repo = BookmarkRepo::new(pool);

    let input = CreateBookmark {
        title: "Rust".to_string(),
        url: "https://rust-lang.org".to_string(),
        description: Some("A systems language".to_string()),
    };
    let created = repo.create(input, uid).await.unwrap();
    let fetched = repo.get_by_id(created.id, uid).await.unwrap();

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.owner_id, uid);
    assert_eq!(fetched.title, "Rust");
    assert_eq!(fetched.url, "https://rust-lang.org");
    assert_eq!(fetched.description, Some("A systems language".to_string()));
}

#[sqlx::test]
async fn get_by_id_missing_returns_not_found(pool: PgPool) {
    let uid = seed_user(&pool, "missing@test.com").await;
    let repo = BookmarkRepo::new(pool);

    let result = repo.get_by_id(Uuid::new_v4(), uid).await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[sqlx::test]
async fn delete_removes_bookmark(pool: PgPool) {
    let uid = seed_user(&pool, "delete@test.com").await;
    let repo = BookmarkRepo::new(pool);

    let created = repo
        .create(
            CreateBookmark {
                title: "Temp".to_string(),
                url: "https://example.com".to_string(),
                description: None,
            },
            uid,
        )
        .await
        .unwrap();

    assert!(repo.delete(created.id, uid).await.unwrap());

    assert!(matches!(
        repo.get_by_id(created.id, uid).await,
        Err(AppError::NotFound(_))
    ));

    assert!(!repo.delete(created.id, uid).await.unwrap());
}

#[sqlx::test]
async fn list_returns_all_bookmarks(pool: PgPool) {
    let uid = seed_user(&pool, "list@test.com").await;
    let repo = BookmarkRepo::new(pool);

    for title in ["One", "Two", "Three"] {
        repo.create(
            CreateBookmark {
                title: title.to_string(),
                url: format!("https://example.com/{title}"),
                description: None,
            },
            uid,
        )
        .await
        .unwrap();
    }

    let all = repo.get_all(uid).await.unwrap();

    assert_eq!(all.len(), 3);
    let titles: Vec<&str> = all.iter().map(|b| b.title.as_str()).collect();
    assert!(titles.contains(&"One"));
    assert!(titles.contains(&"Two"));
    assert!(titles.contains(&"Three"));
}

#[sqlx::test]
async fn update_changes_fields(pool: PgPool) {
    let uid = seed_user(&pool, "update@test.com").await;
    let repo = BookmarkRepo::new(pool);

    let created = repo
        .create(
            CreateBookmark {
                title: "Old title".to_string(),
                url: "https://old.example.com".to_string(),
                description: Some("old".to_string()),
            },
            uid,
        )
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
            uid,
        )
        .await
        .unwrap();

    assert_eq!(updated.title, "New title");
    assert_eq!(updated.description, Some("new".to_string()));
    assert_eq!(updated.url, "https://old.example.com");
}

// ----- HTTP-level bookmark tests (now require a bearer token) -----

#[sqlx::test]
async fn http_create_returns_ok(pool: PgPool) {
    let app = test_app(pool);
    let token = register_and_login(&app, "httpcreate@test.com", "pw12345").await;

    let req = auth_json(
        "POST",
        "/bookmarks",
        json!({
            "title": "Via HTTP",
            "url": "https://http.example.com",
            "description": "made over http"
        }),
        &token,
    );

    let res = send(&app, req).await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[sqlx::test]
async fn http_get_missing_returns_not_found(pool: PgPool) {
    let app = test_app(pool);
    let token = register_and_login(&app, "httpget@test.com", "pw12345").await;

    let req = auth_empty("GET", &format!("/bookmarks/{}", Uuid::new_v4()), &token);
    let res = send(&app, req).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn http_delete_returns_no_content(pool: PgPool) {
    let app = test_app(pool);
    let token = register_and_login(&app, "httpdelete@test.com", "pw12345").await;

    // Create through the authenticated route so owner_id matches the token.
    let created: Bookmark = body_json(
        send(
            &app,
            auth_json(
                "POST",
                "/bookmarks",
                json!({
                    "title": "To delete",
                    "url": "https://delete.example.com",
                    "description": null
                }),
                &token,
            ),
        )
        .await,
    )
    .await;

    let req = auth_empty("DELETE", &format!("/bookmarks/{}", created.id), &token);
    let res = send(&app, req).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn http_list_returns_all(pool: PgPool) {
    let app = test_app(pool);
    let token = register_and_login(&app, "httplist@test.com", "pw12345").await;

    for title in ["A", "B"] {
        let res = send(
            &app,
            auth_json(
                "POST",
                "/bookmarks",
                json!({
                    "title": title,
                    "url": format!("https://example.com/{title}"),
                    "description": null
                }),
                &token,
            ),
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    let res = send(&app, auth_empty("GET", "/bookmarks", &token)).await;
    assert_eq!(res.status(), StatusCode::OK);

    let bookmarks: Vec<Bookmark> = body_json(res).await;
    assert_eq!(bookmarks.len(), 2);
}

// ----- auth tests -----

#[sqlx::test]
async fn register_returns_user_without_password(pool: PgPool) {
    let app = test_app(pool);

    let res = send(
        &app,
        json_request(
            "POST",
            "/register",
            json!({ "email": "reg@test.com", "password": "pw12345" }),
        ),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let text = body_text(res).await;
    assert!(text.contains("reg@test.com"));
    assert!(
        !text.contains("password"),
        "password/hash must not leak: {text}"
    );

    let v: Value = serde_json::from_str(&text).unwrap();
    assert!(v["id"].as_str().is_some());
}

#[sqlx::test]
async fn register_duplicate_email_conflict(pool: PgPool) {
    let app = test_app(pool);
    let body = json!({ "email": "dup@test.com", "password": "pw12345" });

    let first = send(&app, json_request("POST", "/register", body.clone())).await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = send(&app, json_request("POST", "/register", body)).await;
    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[sqlx::test]
async fn login_succeeds(pool: PgPool) {
    let app = test_app(pool);
    send(
        &app,
        json_request(
            "POST",
            "/register",
            json!({ "email": "login@test.com", "password": "pw12345" }),
        ),
    )
    .await;

    let res = send(
        &app,
        json_request(
            "POST",
            "/login",
            json!({ "email": "login@test.com", "password": "pw12345" }),
        ),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let v: Value = body_json(res).await;
    assert!(!v["token"].as_str().unwrap().is_empty());
}

#[sqlx::test]
async fn login_wrong_password_unauthorized(pool: PgPool) {
    let app = test_app(pool);
    send(
        &app,
        json_request(
            "POST",
            "/register",
            json!({ "email": "wrongpw@test.com", "password": "correct-pw" }),
        ),
    )
    .await;

    let res = send(
        &app,
        json_request(
            "POST",
            "/login",
            json!({ "email": "wrongpw@test.com", "password": "wrong-pw" }),
        ),
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn login_unknown_email_unauthorized(pool: PgPool) {
    let app = test_app(pool);

    let res = send(
        &app,
        json_request(
            "POST",
            "/login",
            json!({ "email": "ghost@test.com", "password": "pw12345" }),
        ),
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn whoami_no_header_unauthorized(pool: PgPool) {
    let app = test_app(pool);

    let req = Request::builder()
        .method("GET")
        .uri("/whoami")
        .body(Body::empty())
        .unwrap();
    let res = send(&app, req).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn whoami_garbage_token_unauthorized(pool: PgPool) {
    let app = test_app(pool);

    let res = send(&app, auth_empty("GET", "/whoami", "not.a.jwt")).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn whoami_missing_bearer_prefix_unauthorized(pool: PgPool) {
    let app = test_app(pool);

    let req = Request::builder()
        .method("GET")
        .uri("/whoami")
        .header("authorization", "sometoken")
        .body(Body::empty())
        .unwrap();
    let res = send(&app, req).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn whoami_valid_token_ok(pool: PgPool) {
    let app = test_app(pool);
    let token = register_and_login(&app, "whoami@test.com", "pw12345").await;

    let res = send(&app, auth_empty("GET", "/whoami", &token)).await;
    assert_eq!(res.status(), StatusCode::OK);

    let text = body_text(res).await;
    assert!(text.starts_with("you are "), "unexpected body: {text}");
}

// ----- cross-user ownership -----

#[sqlx::test]
async fn ownership_isolation(pool: PgPool) {
    let app = test_app(pool);
    let token_a = register_and_login(&app, "alice@test.com", "pw12345").await;
    let token_b = register_and_login(&app, "bob@test.com", "pw12345").await;

    // A creates a bookmark.
    let created: Bookmark = body_json(
        send(
            &app,
            auth_json(
                "POST",
                "/bookmarks",
                json!({
                    "title": "Alice's secret",
                    "url": "https://alice.example.com",
                    "description": null
                }),
                &token_a,
            ),
        )
        .await,
    )
    .await;

    // A sees exactly one; B sees none.
    let a_list: Vec<Bookmark> =
        body_json(send(&app, auth_empty("GET", "/bookmarks", &token_a)).await).await;
    assert_eq!(a_list.len(), 1);
    let b_list: Vec<Bookmark> =
        body_json(send(&app, auth_empty("GET", "/bookmarks", &token_b)).await).await;
    assert_eq!(b_list.len(), 0);

    // B cannot touch A's bookmark, even knowing the id -> 404 (existence not leaked).
    let path = format!("/bookmarks/{}", created.id);
    assert_eq!(
        send(&app, auth_empty("GET", &path, &token_b))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            &app,
            auth_json("PATCH", &path, json!({ "title": "hacked" }), &token_b)
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, auth_empty("DELETE", &path, &token_b))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );

    // A can delete their own.
    assert_eq!(
        send(&app, auth_empty("DELETE", &path, &token_a))
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
}
