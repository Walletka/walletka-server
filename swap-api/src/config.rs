use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct SwapApiConfig {
    pub lightning_node_endpoint: String,
    pub mnemonic: String,
    pub swap_api_port: u16,
}
