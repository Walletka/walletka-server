use amqprs::channel::{
    BasicAckArguments, BasicConsumeArguments, Channel, QueueBindArguments, QueueDeclareArguments,
};
use amqprs::consumer::AsyncConsumer;
use amqprs::Deliver;
use amqprs::{channel::BasicPublishArguments, connection::Connection, BasicProperties};
use anyhow::{bail, Result};
use async_trait::async_trait;
use log::warn;
use tokio::sync::Notify;

use crate::messages::LightningNodeEvent;
use crate::{config::RabbitMqConfig, rabbitmq};

/// Publisher sends events to subscribers (listeners).
pub struct LightningNodeEvents {
    pub config: RabbitMqConfig,
    pub connection: Connection,
    pub channel: Channel,
}

impl LightningNodeEvents {
    pub async fn new(config: RabbitMqConfig) -> Result<Self> {
        let connection = rabbitmq::get_rabbitmq_connection(
            &config.rabbitmq_host,
            config.rabbitmq_port,
            &config.rabbitmq_username,
            &config.rabbitmq_password,
        )
        .await?;

        let channel = connection.open_channel(None).await.unwrap();
        rabbitmq::ensure_exchange_created(&channel, &config.lightning_node_exchange, "fanout")
            .await?;

        Ok(Self {
            config,
            connection,
            channel,
        })
    }

    pub async fn notify(&self, event: LightningNodeEvent, routing_key: &str) {
        let args = BasicPublishArguments::new(&self.config.lightning_node_exchange, routing_key);

        let content = serde_json::json!(event).to_string().into_bytes();

        self.channel
            .basic_publish(BasicProperties::default(), content, args)
            .await
            .unwrap();
    }

    pub async fn subscribe_received_payments<F>(&self, queue: &str, callback: F) -> Result<()>
    where
        F: PaymentReceivedProcessor + Send + Sync + 'static,
    {
        // declare a queue
        let (queue_name, _, _) = self
            .channel
            .queue_declare(QueueDeclareArguments::durable_client_named(queue))
            .await
            .unwrap()
            .unwrap();

        // bind the queue to exchange
        let rounting_key = "*";
        let exchange_name = &self.config.lightning_node_exchange; //"walletka.lightning-node";
        self.channel
            .queue_bind(QueueBindArguments::new(
                &queue_name,
                exchange_name,
                rounting_key,
            ))
            .await
            .unwrap();

        if !self.channel.is_connection_open() {
            bail!("Connection is closed");
        }

        let args = BasicConsumeArguments::new(queue, "");

        self.channel
            .basic_consume(WalletkaNodePaymentsConsumer { callback }, args)
            .await
            .unwrap();

        let guard = Notify::new();
        guard.notified().await;

        Ok(())
    }
}

#[async_trait]
pub trait PaymentReceivedProcessor {
    async fn payment_received_callback(&self, payment_hash: String, amount_msat: u64)
        -> Result<()>;
}

pub struct WalletkaNodePaymentsConsumer<F>
where
    F: PaymentReceivedProcessor + Send + Sync + 'static,
{
    callback: F,
}

#[async_trait]
impl<F> AsyncConsumer for WalletkaNodePaymentsConsumer<F>
where
    F: PaymentReceivedProcessor + Send + Sync + 'static,
{
    async fn consume(
        &mut self,
        channel: &amqprs::channel::Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let content = String::from_utf8(content).unwrap();

        let payment: Result<crate::lightning_node_events::LightningNodeEvent, serde_json::Error> =
            serde_json::from_str(&content);

        let ack_args = BasicAckArguments::new(deliver.delivery_tag(), false);
        match payment {
            Ok(payment) => match payment {
                LightningNodeEvent::PaymentReceived {
                    payment_hash,
                    amount_msat,
                } => {
                    match self
                        .callback
                        .payment_received_callback(payment_hash, amount_msat)
                        .await
                    {
                        Ok(_) => {
                            channel.basic_ack(ack_args).await.unwrap();
                        }
                        Err(_) => {}
                    }
                }
                _ => {
                    channel.basic_ack(ack_args).await.unwrap();
                }
            },
            Err(err) => {
                warn!("Error: {}", err)
            }
        }
    }
}
