use anyhow::Result;
use events::lightning_node_events::{LightningNodeEvents, PaymentReceivedProcessor};
use lightning_node_client::proto::node_client::NodeClient;
use log::{info, warn};
use tonic::{async_trait, transport::Channel};

struct PaymentReceivedCallback {}

pub struct PaymentReceivedService {
    pub client: NodeClient<Channel>,
}

impl PaymentReceivedService {
    pub fn new(client: NodeClient<Channel>) -> Self {
        Self { client }
    }

    pub fn subscribe(&self, events: LightningNodeEvents) {
        tokio::spawn(async move {
            info!("Subscribing lightning payments");

            events
                .subscribe_received_payments(
                    "walletka.cashu.received_payments",
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
        payment_hash: String,
        amount_msat: u64,
    ) -> Result<(), anyhow::Error> {
        info!("Final callback, payment received!");
        Ok(())
    }
}
