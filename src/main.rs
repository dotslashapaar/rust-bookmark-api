use std::env;

use bookmarks_api::{
    db::bookmark::BookmarkRepo, handlers::bookmark::AppState, routes::bookmark::bookmark_routes,
};
use dotenvy::dotenv;
use sqlx::PgPool;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    let repo = BookmarkRepo::new(pool);
    let state = AppState {
        bookmark_repo: repo,
    };

    let app = bookmark_routes().with_state(state);

    let listner = TcpListener::bind("0.0.0.0:3000").await?;
    println!("Listening on {}", listner.local_addr()?);
    axum::serve(listner, app).await?;

    Ok(())
}
