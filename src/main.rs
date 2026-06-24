use std::env;

use bookmarks_api::{db, error, handlers, models, routes};
use dotenvy::dotenv;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;
    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await?;
    println!("db says {}", row.0);

    Ok(())
}
