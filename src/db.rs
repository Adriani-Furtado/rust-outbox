use std::env;

use crate::models::OutboxMessages;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use mockall::automock;
use sqlx::{
    mysql::{MySqlConnectOptions, MySqlPoolOptions},
    ConnectOptions, Connection, MySqlConnection,
};

#[automock]
#[async_trait]
pub trait Database {
    async fn get_messages(&self) -> Result<Vec<OutboxMessages>, sqlx::Error>;
    async fn complete_message(&self, id: &str, completed_at: NaiveDateTime) -> bool;
    async fn fail_message(&self, id: &str, error: &str, time: NaiveDateTime) -> bool;
}

pub struct MySqlDatabase {
    pool: sqlx::MySqlPool,
}

impl MySqlDatabase {
    pub async fn new(url: String) -> Self {
        let db_url = env::var("DATABASE_URL").unwrap_or(url);

        let connection = MySqlConnectOptions::new()
            .host("localhost")
            .port(3306)
            .username("root")
            .database("payment_provider_starling")
            .disable_statement_logging()
            .clone();

        let pool = MySqlPoolOptions::new()
            .max_connections(32)
            .connect_with(connection)
            .await
            .expect("Could not acquire Database connection");

        MySqlDatabase { pool }
    }
}

#[async_trait]
impl Database for MySqlDatabase {
    async fn get_messages(&self) -> Result<Vec<OutboxMessages>, sqlx::Error> {
        let start_time = std::time::Instant::now();
        let result = sqlx::query_as!(
            OutboxMessages,
            "SELECT * FROM outbox_messages WHERE completed_at is NULL"
        )
        .fetch_all(&self.pool)
        .await;

        let elapsed = start_time.elapsed();
        log::info!("DB select query took: {:?}", elapsed);
        result
    }

    async fn complete_message(&self, id: &str, completed_at: NaiveDateTime) -> bool {
        let start_time = std::time::Instant::now();
        let result = sqlx::query(
            r#"
                UPDATE outbox_messages
                SET completed_at = ?
                WHERE uuid = ?
                "#,
        )
        .bind(completed_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .is_ok();

        let elapsed = start_time.elapsed();
        log::info!("DB update query took: {:?}", elapsed);
        result
    }
    async fn fail_message(&self, id: &str, error: &str, time: NaiveDateTime) -> bool {
        sqlx::query(
            r#"
                UPDATE outbox_messages
                SET last_error = ?, failed_at = ?
                WHERE uuid = ?
                "#,
        )
        .bind(error)
        .bind(time)
        .bind(id)
        .execute(&self.pool)
        .await
        .is_ok()
    }
}
