use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RequestMintParams {
    pub amount: u64,
}

#[derive(Deserialize)]
pub struct MintParams {
    pub hash: Option<cashu_sdk::Sha256>,
    pub payment_hash: Option<cashu_sdk::Sha256>,
}

#[derive(Deserialize)]
pub struct FaucetQueryParams {
    pub amount: u64,
}

#[derive(Serialize)]
pub struct FaucetResponse {
    pub token: String,
}
