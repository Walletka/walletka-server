use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct CashuApiConfig {
    pub lightning_node_endpoint: String,
    pub cashu_mint_url: String,
    pub cashu_api_port: u16,
    pub cashu_grpc_port: u16,
}
