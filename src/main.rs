#![allow(unused_imports)]
#![allow(unused_imports)]

use std::{error::Error, fmt::Display, future::Future, rc::Rc, sync::Arc, time::Duration};

use clokwerk::Interval::*;
use clokwerk::*;
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
    db::{Database, MySqlDatabase},
    models::OutboxMessages,
    rabbit_publisher::rabbit_publisher::{Publisher, RabbitPublisher},
};
use sqlx::mysql::MySqlPoolOptions;
use std::env;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut scheduler = AsyncScheduler::new();

    info!(target: "main", "App started");

    let publisher = Arc::new(RabbitPublisher::with_connection("localhost", 5672).await);
    let db = Arc::new(
        MySqlDatabase::new("mysql://root@localhost:3306/payment_provider_starling".to_string())
            .await,
    );

    scheduler.every(5.seconds()).run(move || {
        let publisher = Arc::clone(&publisher);
        let db = Arc::clone(&db);
        async move { run_outbox(&*publisher, &*db).await }
    });

    loop {
        scheduler.run_pending().await;
    }
}

async fn run_outbox<Pub: Publisher, DB: Database>(publisher: &Pub, db: &DB) {
    log::info!("Running outbox");
    let messages = db.get_messages().await.unwrap_or_else(|e| {
        log::error!("Failed to fetch messages from the DB {}", e.to_string());
        vec![]
    });
    for message in messages {
        log::info!("Publishing message: {}", message);
        match publisher.publish_message(&message).await {
            Ok(_) => {
                log::info!("Message published successfully: {}", message);
                let time = chrono::Utc::now().naive_utc();
                db.complete_message(&message.uuid, time).await;
            }
            Err(e) => {
                log::error!(
                    "Message failed to publish: {} with {}",
                    message,
                    e.to_string()
                );
                let time = chrono::Utc::now().naive_utc();
                db.fail_message(&message.uuid, "e", time).await;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use lapin::message;
    use mockall::predicate;
    use outbox_publisher::{db::MockDatabase, rabbit_publisher::rabbit_publisher::MockPublisher};

    use super::*;

    #[test]
    fn performs_noop_when_fetching_messages_fail() {
        let publisher = MockPublisher::new();
        let mut db = MockDatabase::new();
        db.expect_get_messages()
            .return_once(|| Result::Err(sqlx::Error::RowNotFound));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(run_outbox(&publisher, &db));
        assert!(result == ())
    }

    #[test]
    fn publishes_messages_and_completes_them() {
        let mut publisher = MockPublisher::new();
        let mut db = MockDatabase::new();
        let message = OutboxMessages::default();
        let message2 = message.clone();
        db.expect_get_messages()
            .return_once(move || Result::Ok(vec![message]));

        publisher
            .expect_publish_message()
            .with(predicate::eq(message2))
            .return_once(|_| Ok(()));

        db.expect_complete_message().return_const(true).once();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(run_outbox(&publisher, &db));
        assert!(result == ())
    }

    #[test]
    fn publishes_messages_and_fails_them() {
        let mut publisher = MockPublisher::new();
        let mut db = MockDatabase::new();
        let message = OutboxMessages::default();
        let message2 = message.clone();
        db.expect_get_messages()
            .return_once(move || Result::Ok(vec![message]));

        publisher
            .expect_publish_message()
            .with(predicate::eq(message2))
            .return_once(|_| Result::Err(lapin::Error::ChannelsLimitReached));

        db.expect_fail_message().return_const(true).once();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(run_outbox(&publisher, &db));
        assert!(result == ())
    }
}
