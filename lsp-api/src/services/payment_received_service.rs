use anyhow::Result;
use events::lightning_node_events::{LightningNodeEvents, PaymentReceivedProcessor};
use lightning_node_client::LightningNodeGrpcClient;
use log::{info, warn};
use tonic::async_trait;

struct PaymentReceivedCallback {}

pub struct PaymentReceivedService {
    pub client: LightningNodeGrpcClient,
}

impl PaymentReceivedService {
    pub fn new(client: LightningNodeGrpcClient) -> Self {
        Self { client }
    }

    pub fn subscribe(&self, events: LightningNodeEvents) {
        tokio::spawn(async move {
            info!("Subscribing lightning payments");

            events
                .subscribe_received_payments(
                    "walletka.lsp.received_payments",
                    PaymentReceivedCallback {},
                )
                .await
                .unwrap();

            warn!("Subscribing payments end!");
        });
    }
}

#[async_trait]
impl PaymentReceivedProcessor for PaymentReceivedCallback {
    async fn payment_received_callback(
        &self,
        _: String,
        _: u64,
    ) -> Result<(), anyhow::Error> {
        info!("Final callback, payment received!");
        // Todo
        Ok(())
    }
}
