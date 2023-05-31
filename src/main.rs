use std::{error::Error, time::Duration};

use log::info;
use outbox_publisher::{db::MySqlDatabase, outbox::run_outbox, rabbit_publisher::RabbitPublisher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!(target: "main", "App started");

    let publisher = RabbitPublisher::with_connection("localhost", 5672).await;
    let db =
        MySqlDatabase::new("mysql://root@localhost:3306/payment_provider_starling".to_string())
            .await;

    loop {
        let start_time = std::time::Instant::now();
        run_outbox(&publisher, &db).await?;
        let elapsed = start_time.elapsed();
        info!(target: "main", "Outbox run took: {:?}", elapsed);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
