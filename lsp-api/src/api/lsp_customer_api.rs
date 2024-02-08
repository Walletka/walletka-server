use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension, Json,
};
use database::surrealdb::engine::remote::ws::Client;
use lightning_invoice::Bolt11Invoice;
use lightning_node_client::{proto::CreateBolt11InvoiceRequest, LightningNodeGrpcClient};
use log::info;
use std::{str::FromStr, sync::Arc};

use crate::{
    config::LspConfig,
    entity::{LspCustomer, LspCustomerConfig},
    services::lsp_customer_service::LspCustomerService,
};

use super::models::{GetInvoiceParams, GetInvoiceResponse, LspSignUpRequest};

pub async fn lsp_signup(
    lsp_customer_service: Extension<Arc<LspCustomerService<Client>>>,
    Json(body): Json<LspSignUpRequest>,
) -> Result<Json<LspCustomer>, StatusCode> {
    info!("Signing up new lsp customer");

    let node_id = match body.node_id {
        Some(node_id) => {
            Some(bitcoin::PublicKey::from_str(&node_id).expect("node_id is not valid pubkey"))
        }
        None => None,
    };

    match lsp_customer_service
        .get_customer_by_npub(body.nostr_pubkey.clone())
        .await
    {
        Some(customer) => {
            info!(
                "Customer with same npub and alias {} already exists",
                customer.alias
            );
            return Ok(Json(customer));
        }
        None => {}
    }

    match lsp_customer_service
        .create_customer(body.nostr_pubkey, node_id)
        .await
    {
        Ok(customer) => {
            info!("Lsp customer created with alias {}", customer.alias);
            Ok(Json(customer))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update_config(
    lsp_customer_service: Extension<Arc<LspCustomerService<Client>>>,
    Path(alias): Path<String>,
    Json(body): Json<LspCustomerConfig>,
) -> Result<Json<LspCustomerConfig>, StatusCode> {
    let res = lsp_customer_service
        .update_customer_lsp_config(alias.as_str(), body)
        .await;

    match res {
        Ok(config) => Ok(Json(config)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_invoice(
    lsp_customer_service: Extension<Arc<LspCustomerService<Client>>>,
    config: Extension<LspConfig>,
    query: Query<GetInvoiceParams>,
    Path(alias): Path<String>,
) -> Result<Json<GetInvoiceResponse>, StatusCode> {
    let customer = lsp_customer_service
        .get_customer_by_alias(alias)
        .await
        .unwrap();

    let mut node_client =
        LightningNodeGrpcClient::new(config.lightning_node_endpoint.clone(), true)
            .await
            .unwrap();

    let invoice_res = node_client
        .create_bolt11_invoice(CreateBolt11InvoiceRequest {
            amount_msat: query.amount.unwrap_or(0),
            expiry_secs: 36000,
            description: "Walletka lsp invoice".to_string(),
        })
        .await
        .unwrap()
        .into_inner();

    lsp_customer_service
        .store_invoice(
            customer.alias,
            Bolt11Invoice::from_str(&invoice_res.invoice).unwrap(),
        )
        .await
        .unwrap();

    Ok(Json(GetInvoiceResponse {
        pr: invoice_res.invoice,
        success_action: None,
        routes: vec![],
    }))
}
