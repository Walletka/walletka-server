use std::sync::Arc;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Extension, Router,
};
use config::CashuApiConfig;
use database::{config::SurrealDbConfig, init_db};
use dotenv::dotenv;
use events::config::RabbitMqConfig;
use lightning_node_client::get_lightning_node_client;
use log::info;
use repositories::cashu_repository::CashuMintReporitory;
use services::payment_received_service::PaymentReceivedService;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use crate::api::cashu_api;

mod api;
mod cashu;
mod config;
mod repositories;
mod services;
mod types;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = envy::from_env::<CashuApiConfig>().unwrap();
    let rabbitmq_config = envy::from_env::<RabbitMqConfig>().unwrap();
    let db_config = envy::from_env::<SurrealDbConfig>().unwrap();

    let events = events::lightning_node_events::LightningNodeEvents::new(rabbitmq_config).await?;
    let subscribe_node_client =
        get_lightning_node_client(config.lightning_node_endpoint.clone(), true).await?;

    let database = init_db(db_config, "walletka", "cashu").await?;

    let payment_received_service = PaymentReceivedService::new(subscribe_node_client);
    payment_received_service.subscribe(events);

    let cashu_repository = Arc::new(CashuMintReporitory::new(database));

    let cashu = Arc::new(
        cashu::CashuService::init(config.cashu_mint_url.clone(), cashu_repository)
            .await
            .unwrap(),
    );

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods(Any)
        .allow_headers(Any)
        // allow requests from any origin
        .allow_origin(Any);

    let rest_app = Router::new()
        .route("/:mint_id/mint", get(cashu_api::get_request_mint))
        .route("/:mint_id/mint", post(cashu_api::post_mint))
        .route("/:mint_id/info", get(cashu_api::info))
        .route("/:mint_id/keys", get(cashu_api::keys))
        .route("/:mint_id/keysets", get(cashu_api::keysets))
        .route("/:mint_id/split", post(cashu_api::post_split))
        .route("/:mint_id/melt", post(cashu_api::post_melt))
        // todo .route("/check", post().to(cashu_api::post_check))
        .route("/:mint_id/checkfees", post(cashu_api::post_check_fee))
        .route("/:mint_id/faucet", get(cashu_api::faucet))
        .layer(ServiceBuilder::new().layer(cors))
        .layer(Extension(cashu.clone()))
        .layer(Extension(Arc::new(config.clone())))
        .into_make_service();

    //tokio::spawn(async move {
    info!(
        "Starting rest api server at 0.0.0.0:{}",
        config.cashu_api_port
    );
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.cashu_api_port))
        .await
        .unwrap();
    axum::serve(listener, rest_app).await.unwrap();
    //});

    Ok(())
}
