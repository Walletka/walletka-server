use std::sync::Arc;

use anyhow::Result;
use axum::{
    routing::{get, post, put},
    Extension, Router,
};
use database::{config::SurrealDbConfig, init_db};
use dotenv::dotenv;
use events::config::RabbitMqConfig;
use lightning_node_client::LightningNodeGrpcClient;
use log::info;
use repository::{
    lsp_customer_repository::LspCustomerRepository, lsp_invoice_repository::LspInvoiceRepository,
};
use services::lsp_customer_service::LspCustomerService;

use crate::{config::LspConfig, services::payment_received_service::PaymentReceivedService};

mod api;
mod client;
mod config;
mod entity;
mod repository;
mod services;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = envy::from_env::<LspConfig>().unwrap();
    let rabbitmq_config = envy::from_env::<RabbitMqConfig>().unwrap();
    let db_config = envy::from_env::<SurrealDbConfig>().unwrap();

    let events = events::lightning_node_events::LightningNodeEvents::new(rabbitmq_config).await?;
    let node_client =
        LightningNodeGrpcClient::new(config.lightning_node_endpoint.clone(), true).await?;

    let nostr_client = client::nostr_client::NostrClient::from_mnemonic(&config.mnemonic, None);
    nostr_client
        .start(&config.nostr_default_relay)
        .await
        .unwrap();

    let database = init_db(db_config, "walletka", "lsp").await?;

    let payment_received_service = PaymentReceivedService::new(node_client);
    payment_received_service.subscribe(events);

    let customer_repo = LspCustomerRepository::new(database.clone());
    let invoice_repo = LspInvoiceRepository::new(database.clone());

    let lsp_service = Arc::new(LspCustomerService::new(
        customer_repo,
        invoice_repo,
        config.default_cashu_endpoint.clone(),
        config.lsp_cashu_mint.clone(),
        nostr_client,
    ));

    info!(
        "Starting rest api server at 0.0.0.0:{}",
        config.lsp_api_port
    );

    let app = Router::new()
        .route("/api/lsp/signup", post(api::lsp_customer_api::lsp_signup))
        .route("/api/lsp/config", put(api::lsp_customer_api::update_config))
        .route(
            "/api/lsp/invoice/:alias",
            get(api::lsp_customer_api::get_invoice),
        )
        .route("/.well-known/nostr.json", get(api::nostr_api::nip05))
        .layer(Extension(lsp_service))
        .layer(Extension(config.clone()));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.lsp_api_port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
