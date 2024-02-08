use serde::Deserialize;

#[derive(Deserialize)]
pub struct RabbitMqConfig {
    pub rabbitmq_host: String,
    pub rabbitmq_port: u16,
    pub rabbitmq_username: String,
    pub rabbitmq_password: String,
    pub lightning_node_exchange: String
}
