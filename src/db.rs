use std::env;

use crate::models::OutboxMessages;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use mockall::automock;
use sqlx::mysql::MySqlPoolOptions;

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
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("Could not acquire Database connection");

        MySqlDatabase { pool }
    }
}

#[async_trait]
impl Database for MySqlDatabase {
    async fn get_messages(&self) -> Result<Vec<OutboxMessages>, sqlx::Error> {
        sqlx::query_as!(
            OutboxMessages,
            "SELECT * FROM outbox_messages WHERE completed_at is NULL"
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn complete_message(&self, id: &str, completed_at: NaiveDateTime) -> bool {
        sqlx::query(
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
        .is_ok()
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
