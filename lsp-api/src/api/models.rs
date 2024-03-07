use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct LspSignUpRequest {
    pub node_id: Option<String>,
    pub nostr_pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInvoiceParams {
    pub amount: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetInvoiceResponse {
    pub pr: String,
    // TODO: find out proper type
    pub success_action: Option<String>,
    // TODO: find out proper type
    pub routes: Vec<String>,
}

// Nostr
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip05Params {
    pub name: Option<String>,
}

