use anyhow::Result;
use axum::{Extension, Router};
use config::SwapApiConfig;
use database::{config::SurrealDbConfig, init_db};
use dotenv::dotenv;
use events::config::RabbitMqConfig;
use lightning_node_client::get_lightning_node_client;
use log::info;

mod config;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = envy::from_env::<SwapApiConfig>().unwrap();

    let rabbitmq_config = envy::from_env::<RabbitMqConfig>().unwrap();
    let db_config = envy::from_env::<SurrealDbConfig>().unwrap();
    
    let database = init_db(db_config, "walletka", "lsp").await?;

    let events = events::lightning_node_events::LightningNodeEvents::new(rabbitmq_config).await?;
    let node_client =
        get_lightning_node_client(config.lightning_node_endpoint.clone(), true).await?;

    info!(
        "Starting rest api server at 0.0.0.0:{}",
        config.swap_api_port
    );

    let app = Router::new()
        .layer(Extension(config.clone()));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.swap_api_port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
