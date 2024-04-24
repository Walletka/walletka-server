use serde::{Deserialize, Serialize};
use database::surrealdb::sql::Datetime;
use database::surrealdb::sql::Thing;

#[derive(Debug, Serialize, Deserialize)]
pub struct LspCustomer {
    #[allow(dead_code)]
    pub id: Option<Thing>,
    pub node_id: Option<String>,
    pub npub: Option<String>,
    pub alias: String,
    pub config: LspCustomerConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspInvoice {
    #[allow(dead_code)]
    pub id: Option<Thing>,
    pub payment_hash: String,
    pub bolt11: String,
    pub amount_msat: Option<u64>,
    pub expiration: Datetime,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LspCustomerConfig {
    pub min_channel_size_sat: u64,
    pub include_onchain_fee: bool,
    pub enable_ecash: bool,
    pub max_ecash_receive_sat: u64,
    pub public_channels: bool,
}

impl Default for LspCustomerConfig {
    fn default() -> Self {
        Self {
            min_channel_size_sat: 20_000,
            include_onchain_fee: false,
            enable_ecash: true,
            max_ecash_receive_sat: 210_000_000_000,
            public_channels: true, // todo
        }
    }
}
