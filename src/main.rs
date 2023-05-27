#![allow(unused_imports)]
#![allow(unused_imports)]

use std::{error::Error, fmt::Display, future::Future, rc::Rc, sync::Arc, time::Duration};

use clokwerk::Interval::*;
use clokwerk::*;
use db::Database;
use dotenvy::dotenv;
use lapin::{
    message::DeliveryResult,
    options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions},
    publisher_confirm::{Confirmation, PublisherConfirm},
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties,
};
use log::{debug, error, info, log_enabled, Level};
use outbox_publisher::{
    models::OutboxMessages,
    rabbit_publisher::rabbit_publisher::{Publisher, RabbitPublisher},
};
use sqlx::mysql::MySqlPoolOptions;
use std::env;

use crate::db::MySqlDatabase;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut scheduler = AsyncScheduler::new();

    info!(target: "main", "App started");

    let publisher = Arc::new(RabbitPublisher::with_connection("localhost", 5672).await);
    let db = Arc::new(
        MySqlDatabase::new("mysql://root@localhost:3306/payment_provider_starling".to_string()).await,
    );

    scheduler.every(5.seconds()).run(move || {
        let publisher = Arc::clone(&publisher);
        let db = Arc::clone(&db);
        async move {
            run_outbox(&publisher, &db).await
        }
    });

    loop {
        scheduler.run_pending().await;
    }
}

async fn run_outbox<P: Publisher, DB: Database>(publisher: &Arc<P>, db: &Arc<DB>) {
    log::info!("Running outbox");
    let messages = db.get_messages().await;
    for message in messages {
        log::info!("Publishing message: {}", message);
        let result = publisher.publish_message(&message).await;
        if result.is_ok() {
            log::info!("Message published successfully: {}", message);
            let time = chrono::Utc::now().naive_utc();
            db.complete_message(&message.uuid, time).await;
        } else {
            log::error!("Message failed to publish: {}", message);
            let time = chrono::Utc::now().naive_utc();
            db.fail_message(&message.uuid, "e", time).await;
        }
    }
}

mod db {
    use std::env;

    use async_trait::async_trait;
    use chrono::NaiveDateTime;
    use outbox_publisher::models::OutboxMessages;
    use sqlx::mysql::MySqlPoolOptions;

    #[async_trait]
    pub trait Database {
        async fn get_messages(&self) -> Vec<OutboxMessages>;
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
        async fn get_messages(&self) -> Vec<OutboxMessages> {
            

            sqlx::query_as!(
                OutboxMessages,
                "SELECT * FROM outbox_messages WHERE completed_at is NULL"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap()
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
}

// async fn establish_connection() {
//     let db_url = "mysql://root@127.0.0.1:3306/payment_provider_starling";
//     let pool = MySqlPoolOptions::new()
//         .max_connections(5)
//         .connect(db_url)
//         .await
//         .expect("Could not acquire Database connection");

//     let result = sqlx::query_as!(
//         OutboxMessages,
//         "SELECT * FROM outbox_messages"
//     )
//     .fetch_one(&pool)
//     .await
//     .unwrap();

//     info!("Retrieved result from table {}", result)
// }
