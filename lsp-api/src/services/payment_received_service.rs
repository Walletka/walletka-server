use std::sync::Arc;

use anyhow::Result;
use database::surrealdb::Connection;
use events::lightning_node_events::{LightningNodeEvents, PaymentReceivedProcessor};
use lightning_node_client::proto::node_client::NodeClient;
use log::{info, warn};
use tonic::{async_trait, transport::Channel};

use super::lsp_customer_service::LspCustomerService;

#[derive(Clone)]
struct PaymentReceivedCallback<C>
where
    C: Connection,
{
    client: NodeClient<Channel>,
    lsp_customer_service: Arc<LspCustomerService<C>>,
}

pub struct PaymentReceivedService<C>
where
    C: Connection,
{
    client: NodeClient<Channel>,
    lsp_customer_service: Arc<LspCustomerService<C>>,
}

impl<C> PaymentReceivedService<C>
where
    C: Connection,
{
    pub fn new(
        client: NodeClient<Channel>,
        lsp_customer_service: Arc<LspCustomerService<C>>,
    ) -> Self {
        Self {
            client,
            lsp_customer_service,
        }
    }

    pub fn subscribe(&self, events: LightningNodeEvents) {
        let callback = PaymentReceivedCallback {
            client: self.client.clone(),
            lsp_customer_service: self.lsp_customer_service.clone(),
        };

        tokio::spawn(async move {
            info!("Subscribing lightning payments");

            events
                .subscribe_received_payments("walletka.lsp.received_payments", callback)
                .await
                .unwrap();

            warn!("Subscribing payments end!");
        });
    }
}

#[async_trait]
impl<C> PaymentReceivedProcessor for PaymentReceivedCallback<C>
where
    C: Connection,
{
    async fn payment_received_callback(
        &self,
        payment_hash: String,
        amount_msat: u64,
    ) -> Result<(), anyhow::Error> {
        let mut client = self.client.clone();

        info!("Final callback, payment received!");

        match self
            .lsp_customer_service
            .handle_paid_invoice(&mut client, payment_hash, amount_msat)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                warn!("Cannot handle paid invoice");
                Err(err)
            }
        }
    }
}
