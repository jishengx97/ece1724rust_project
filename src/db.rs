use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::time::Duration;

// Database connection manager
pub struct Database {
    pub pool: MySqlPool,
}

impl Database {
    // Create a new database connection pool
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(3))
            .connect(database_url)
            .await?;

        Ok(Database { pool })
    }

    // Get a reference to the connection pool
    pub fn get_pool(&self) -> &MySqlPool {
        &self.pool
    }
}