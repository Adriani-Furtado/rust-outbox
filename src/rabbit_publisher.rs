pub mod rabbit_publisher {

    use async_trait::async_trait;
    use lapin::{
        options::BasicPublishOptions, BasicProperties,
        Channel, Error,
    };
    

    use crate::models::OutboxMessages;

    // #[automock]
    #[async_trait]
    pub trait Publisher {
        async fn publish_message(&self, message: &OutboxMessages)
            -> Result<(), Error>;
    }

    pub struct RabbitPublisher {
        channel: Channel,
    }

    impl RabbitPublisher {
        pub async fn with_connection(host: &str, port: u16) -> Self {
            log::info!("Connecting to RabbitMQ");
            let uri = format!("amqp://{}:{}", host, port);
            let options = lapin::ConnectionProperties::default()
                .with_executor(tokio_executor_trait::Tokio::current())
                .with_reactor(tokio_reactor_trait::Tokio);

            let connection = lapin::Connection::connect(&uri, options)
                .await
                .expect("could not connect to rabbitmq");
            let channel = connection.create_channel().await.unwrap();

            RabbitPublisher { channel }
        }
    }

    #[async_trait]
    impl Publisher for RabbitPublisher {
        async fn publish_message(
            &self,
            message: &OutboxMessages,
        ) -> Result<(), Error> {
            let channel = self.channel.clone();
            log::info!("Publishing message: {}", message);
            let result = channel.basic_publish(
                &message.exchange,
                &message.routing_key,
                BasicPublishOptions::default(),
                message.payload.as_bytes(),
                BasicProperties::default(),
            );
            result.await.map(|_| ())
        }
    }
}