use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct LspConfig {
    pub lightning_node_endpoint: String,
    pub mnemonic: String,
    pub nostr_default_relay: String,
    pub default_cashu_endpoint: String,
    pub lsp_cashu_mint: String,
    pub lsp_api_port: u16,
}