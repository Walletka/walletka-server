use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct LightningNodeConfig {
    pub lightning_data_dir: String,
    pub lightning_node_port: u16,
    pub lightning_node_grpc_port: u16,
    pub mnemonic: Option<String>,
    pub esplora_server_url: String,
}
