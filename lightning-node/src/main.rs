use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use dotenv::dotenv;
use events::config::RabbitMqConfig;
use log::info;
use tonic::transport::Server;

use crate::server::{node_api::node_server::NodeServer, LightningNodeGrpcServer};

mod config;
mod processor;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let config: config::LightningNodeConfig =
        envy::from_env::<config::LightningNodeConfig>().unwrap();
    let rabbitmq_config = envy::from_env::<RabbitMqConfig>().unwrap();

    info!("Starting Lightning node...");

    let node_processor =
        Arc::new(processor::NodeProcessor::new(config.clone(), rabbitmq_config).await?);
    node_processor.start()?;

    let addr = SocketAddr::from(([0, 0, 0, 0], config.lightning_node_grpc_port));
    let node_service = NodeServer::new(LightningNodeGrpcServer {
        node: node_processor,
    });

    info!(
        "Starting grpc server at :{}",
        config.lightning_node_grpc_port
    );

    Server::builder()
        .accept_http1(true)
        .add_service(tonic_web::enable(node_service))
        .serve(addr)
        .await?;

    Ok(())
}
