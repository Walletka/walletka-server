use anyhow::{bail, Result};
use bitcoin::PublicKey;
use cashu_internal_client::{get_cashu_client, proto::InternalTokenMintRequest};
use chrono::Utc;
use database::surrealdb::{sql::Datetime, Connection};
use lightning_invoice::Bolt11Invoice;
use lightning_node_client::proto::{
    node_client::NodeClient, OpenChannelRequest, SendKeysendPaymentRequest,
};
use log::{debug, info, warn};
use tonic::transport::Channel;

use crate::{
    client::nostr_client::NostrClient,
    entity::{LspCustomer, LspCustomerConfig, LspInvoice},
    repository::{
        lsp_customer_repository::LspCustomerRepository,
        lsp_invoice_repository::LspInvoiceRepository,
    },
    utils,
};

pub struct LspCustomerService<C>
where
    C: Connection,
{
    repository: LspCustomerRepository<C>,
    invoice_repository: LspInvoiceRepository<C>,
    walletka_bank_endpoint: String,
    cashu_mint: String,
    nostr_client: NostrClient,
}

impl<C> LspCustomerService<C>
where
    C: Connection,
{
    pub fn new(
        repository: LspCustomerRepository<C>,
        invoice_repository: LspInvoiceRepository<C>,
        walletka_bank_endpoint: String,
        cashu_mint: String,
        nostr_client: NostrClient,
    ) -> Self {
        Self {
            repository,
            invoice_repository,
            walletka_bank_endpoint,
            cashu_mint,
            nostr_client,
        }
    }

    pub async fn create_customer(
        &self,
        npub: String,
        node_id: Option<PublicKey>,
    ) -> Result<LspCustomer> {
        info!("Creating customer");

        let node_id = match node_id {
            Some(id) => Some(id.to_string()),
            None => None,
        };

        let alias = utils::generate_random_name();

        info!("Generated alias {}", alias);

        let lsp_customer = LspCustomer {
            id: None,
            node_id,
            npub: Some(npub),
            alias,
            config: LspCustomerConfig::default(),
        };

        let res = self.repository.add_customer(lsp_customer).await?;

        Ok(res)
    }

    pub async fn update_customer_lsp_config(
        &self,
        alias: &str,
        config: LspCustomerConfig,
    ) -> Result<LspCustomerConfig> {
        info!("Updating config for {}", alias);

        let res = self
            .repository
            .update_customer_lsp_config(alias, config)
            .await?;
        Ok(res)
    }

    pub async fn get_customers(&self) -> Result<Vec<LspCustomer>> {
        let res = self.repository.get_customers().await?;
        Ok(res)
    }

    pub async fn get_customer_by_alias(&self, alias: String) -> Option<LspCustomer> {
        let res = self.repository.get_customer_by_alias(alias).await.unwrap();
        res
    }

    pub async fn get_customer_by_npub(&self, npub: String) -> Option<LspCustomer> {
        let res = self.repository.get_customer_by_npub(npub).await.unwrap();
        res
    }

    pub async fn store_invoice(&self, alias: String, invoice: Bolt11Invoice) -> Result<LspInvoice> {
        let expiration = Utc::now() + invoice.duration_until_expiry();

        let lsp_invoice = LspInvoice {
            id: None,
            bolt11: invoice.to_string(),
            amount_msat: invoice.amount_milli_satoshis(),
            expiration: Datetime(expiration),
            payment_hash: invoice.payment_hash().to_string(),
        };
        let res = self
            .invoice_repository
            .add_invoice(lsp_invoice, alias)
            .await?;

        Ok(res)
    }

    pub async fn handle_paid_invoice(
        &self,
        node_client: &mut NodeClient<Channel>,
        payment_hash: String,
        amount_msat: u64,
    ) -> Result<()> {
        let customer = self
            .repository
            .get_customer_by_payment_hash(payment_hash.clone())
            .await
            .unwrap();

        let customer = if customer.is_some() {
            customer.unwrap()
        } else {
            debug!("Invoice with payment hash \"{}\" not found", payment_hash);
            return Ok(());
        };

        // Get customer config
        if customer.node_id.is_some() {
            let node_id = customer.node_id.clone().unwrap();

            info!("Sending keysend payment to {}", customer.alias);
            match node_client
                .send_keysend_payment(SendKeysendPaymentRequest {
                    destination: node_id.clone(),
                    amount: amount_msat,
                })
                .await
            {
                Ok(_) => {
                    //let res = res.into_inner();
                    info!("Keysend payment sent!");
                    Ok(())
                }
                Err(_) => {
                    warn!("Keysend payment failed"); // Todo check reason
                    if amount_msat > customer.config.min_channel_size_sat * 1000 {
                        info!(
                            "Opening channel to {} with push amount {} sats",
                            &customer.alias, amount_msat
                        );

                        match node_client
                            .open_channel(OpenChannelRequest {
                                node_id: node_id.clone(),
                                address: "".to_string(),
                                channel_amount_sats: (amount_msat / 1000) * 12 / 10, // Open channel with requested amount + 20%
                                push_to_counterparty_msat: amount_msat,
                                public: customer.config.public_channels,
                            })
                            .await
                        {
                            Ok(_) => {
                                info!("Channel to {} openned successfully", customer.alias.clone());
                                Ok(())
                            }
                            Err(_) => {
                                self.mint_and_send_token(
                                    &customer,
                                    self.cashu_mint.clone(),
                                    amount_msat,
                                )
                                .await
                            }
                        }
                    } else {
                        self.mint_and_send_token(&customer, self.cashu_mint.clone(), amount_msat)
                            .await
                    }
                }
            }
        } else {
            Ok(())
        }
    }

    async fn mint_and_send_token(
        &self,
        lsp_customer: &LspCustomer,
        mint_id: String,
        amount_msat: u64,
    ) -> Result<()> {
        let mut cashu_client = get_cashu_client(self.walletka_bank_endpoint.clone(), false)
            .await
            .unwrap();

        let res = cashu_client
            .internal_token_mint(InternalTokenMintRequest {
                amount_sat: amount_msat,
                service_name: "walletka-lsp".to_string(),
                mint_id,
            })
            .await
            .unwrap()
            .into_inner();

        let customer_npub = match lsp_customer.npub.clone() {
            Some(npub) => npub,
            None => bail!("Customer {} is missing npub!", lsp_customer.alias),
        };

        info!(
            "Sending token to {} over nostr using npub {}",
            lsp_customer.alias, customer_npub
        );

        match self
            .nostr_client
            .send_message(customer_npub, res.token.as_str())
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => bail!(err),
        }
    }
}
