use crate::{db::Database, rabbit_publisher::Publisher};

use std::error::Error;

pub async fn run_outbox<Pub: Publisher, DB: Database>(
    publisher: &Pub,
    db: &DB,
) -> Result<(), Box<dyn Error>> {
    log::info!("Running outbox");
    let messages = db.get_messages().await?;
    for message in messages {
        match publisher.publish_message(&message).await {
            Ok(_) => {
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
    Ok(())
}

#[cfg(test)]
mod test {
    use mockall::predicate;

    use crate::{db::MockDatabase, models::OutboxMessages, rabbit_publisher::MockPublisher};

    use super::*;

    #[test]
    fn performs_noop_when_fetching_messages_fail() {
        let publisher = MockPublisher::new();
        let mut db = MockDatabase::new();
        db.expect_get_messages()
            .return_once(|| Result::Err(sqlx::Error::RowNotFound));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(run_outbox(&publisher, &db)).unwrap_err();
        assert!(result.is::<sqlx::Error>())
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
        let result = rt.block_on(run_outbox(&publisher, &db)).unwrap();
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
        let result = rt.block_on(run_outbox(&publisher, &db)).unwrap();
        assert!(result == ())
    }
}
