use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .max_lifetime(Duration::from_secs(30 * 60))
        .idle_timeout(Duration::from_secs(10 * 60))
        .test_before_acquire(true)
        .connect(database_url)
        .await
}
