use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension, Json,
};
use bitcoin::hashes::{sha256::Hash as Sha256, Hash};
use cashu_sdk::{
    nuts::{
        nut03::RequestMintResponse,
        nut04::MintRequest,
        nut05::{CheckFeesRequest, CheckFeesResponse},
        nut06::SplitRequest,
        //nut07::CheckSpendableRequest,
        nut08::MeltRequest,
        Keys,
        KeysetResponse,
        MeltResponse,
        PostMintResponse,
        SplitResponse,
    },
    types::InvoiceStatus,
    Amount, Bolt11Invoice,
};
use database::surrealdb::engine::remote::ws::Client;
use lightning_node_client::{
    proto::{CreateBolt11InvoiceRequest, PayInvoiceRequest},
    LightningNodeGrpcClient,
};
use log::info;
use std::{fmt::Write, str::FromStr, sync::Arc};

use crate::{
    cashu::CashuService,
    config::CashuApiConfig,
    types::{InvoiceInfo, InvoiceTokenStatus, MintInfo},
};

use super::models::{FaucetQueryParams, FaucetResponse, MintParams, RequestMintParams};

pub async fn info(
    Path(mint_id): Path<String>,
    cashu: Extension<Arc<CashuService<Client>>>,
) -> Result<Json<MintInfo>, StatusCode> {
    let mint = cashu.repository.get_mint(mint_id).await.unwrap();
    let res = MintInfo {
        name: Some(mint.name),
        version: mint.version,
        description: mint.description,
        description_long: mint.description_long,
        contact: mint.contact,
        nuts: mint.nuts,
        motd: mint.motd,
    };
    Ok(Json(res))
}

pub async fn keys(
    Path(mint_id): Path<String>,
    cashu: Extension<Arc<CashuService<Client>>>,
) -> Result<Json<Keys>, StatusCode> {
    let mint = cashu.mints.get(&mint_id).unwrap().lock().await;

    let keys = mint.active_keyset_pubkeys();

    Ok(Json(keys.keys))
}

pub async fn keysets(
    Path(mint_id): Path<String>,
    cashu: Extension<Arc<CashuService<Client>>>,
) -> Result<Json<KeysetResponse>, StatusCode> {
    let mint = cashu.mints.get(&mint_id).unwrap().lock().await;
    let keysets = mint.keysets();

    Ok(Json(keysets))
}

pub async fn get_request_mint(
    Path(mint_id): Path<String>,
    cashu: Extension<Arc<CashuService<Client>>>,
    config: Extension<Arc<CashuApiConfig>>,
    mint_params: Query<RequestMintParams>,
) -> Result<Json<RequestMintResponse>, StatusCode> {
    let mut node_client =
        LightningNodeGrpcClient::new(config.lightning_node_endpoint.clone(), false)
            .await
            .unwrap();

    let invoice = match node_client
        .create_bolt11_invoice(CreateBolt11InvoiceRequest {
            amount_msat: mint_params.amount * 1000,
            expiry_secs: 3600,
            description: "Walletka Cashu invoice".to_string(),
        })
        .await
    {
        Ok(inv) => inv.into_inner(),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    //let hash = String::from_utf8(cashu_sdk::utils::random_hash()).unwrap();
    let hash1 = Sha256::hash(&cashu_sdk::utils::random_hash());
    let bolt11_invoice = Bolt11Invoice::from_str(&invoice.invoice).unwrap();
    let payment_hash =
        cashu_sdk::Sha256::from_str(&bolt11_invoice.payment_hash().to_string().as_str()).unwrap();

    println!("Storing payment hash: {}", payment_hash.to_string());

    let amount = Amount::from_msat(bolt11_invoice.amount_milli_satoshis().unwrap());
    let cashu_invoice = Bolt11Invoice::from_str(&invoice.invoice).unwrap();

    cashu
        .repository
        .add_invoice(&InvoiceInfo {
            payment_hash: payment_hash,
            hash: cashu_sdk::Sha256::from_str(hash1.to_string().as_str()).unwrap(),
            amount,
            status: cashu_sdk::types::InvoiceStatus::Unpaid,
            token_status: InvoiceTokenStatus::NotIssued,
            memo: "".to_string(),
            invoice: cashu_invoice.clone(),
            confirmed_at: None,
            mint_id: Some(mint_id),
        })
        .await
        .unwrap();

    Ok(Json(RequestMintResponse {
        hash: hash1.to_string(),
        pr: cashu_invoice,
    }))
}

pub async fn post_mint(
    Path(mint_id): Path<String>,
    cashu: Extension<Arc<CashuService<Client>>>,
    mint_params: Query<MintParams>,
    payload: Json<MintRequest>,
) -> Result<Json<PostMintResponse>, StatusCode> {
    let hash = match mint_params.hash {
        Some(hash) => hash,
        None => match mint_params.payment_hash {
            Some(hash) => hash,
            None => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
    };

    let invoice = cashu
        .repository
        .get_invoice_info(&hash)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
        .unwrap();

    // debug!("{:?}", invoice);
    let total_amount = payload.total_amount();
    if invoice.amount.to_msat() != total_amount.to_msat() {
        println!("Wrong amount");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    match invoice.status {
        InvoiceStatus::Paid => {}
        InvoiceStatus::Unpaid => {
            println!("Checking");

            let invoice = cashu.repository.get_invoice_info(&hash).await.unwrap();

            match invoice.status {
                InvoiceStatus::Unpaid => return Err(StatusCode::INTERNAL_SERVER_ERROR),
                InvoiceStatus::Expired => return Err(StatusCode::INTERNAL_SERVER_ERROR),
                _ => (),
            }

            println!("Unpaid check: {:?}", invoice.status);
        }
        InvoiceStatus::Expired => {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        InvoiceStatus::InFlight => {}
    }

    let mut mint = cashu.mints.get(&mint_id).unwrap().lock().await;

    let res = match mint.process_mint_request(payload.0) {
        Ok(mint_res) => {
            let mut invoice = cashu
                .repository
                .get_invoice_info(&hash)
                .await
                .map_err(|err| {
                    println!("{}", err);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
                .unwrap();
            invoice.token_status = InvoiceTokenStatus::Issued;

            cashu
                .repository
                .add_invoice(&invoice)
                .await
                .map_err(|err| {
                    println!("{}", err);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
                .unwrap();
            let in_circulation =
                cashu.repository.get_in_circulation(&mint_id).await.unwrap() + invoice.amount;

            cashu
                .repository
                .set_in_circulation(&mint_id, &in_circulation)
                .await
                .ok();

            mint_res
        }
        Err(err) => match cashu.repository.get_invoice_info(&hash).await {
            Ok(_) => {
                println!("{}", err);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Err(err) => {
                println!("{}", err);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        },
    };

    Ok(Json(res))
}

pub async fn post_split(
    Path(mint_id): Path<String>,
    cashu: Extension<Arc<CashuService<Client>>>,
    payload: Json<SplitRequest>,
) -> Result<Json<SplitResponse>, StatusCode> {
    let mut mint = cashu.mints.get(&mint_id).unwrap().lock().await;

    let proofs = payload.proofs.clone();

    match mint.process_split_request(payload.0) {
        Ok(split_response) => {
            cashu
                .repository
                .add_used_proofs(mint_id, &proofs)
                .await
                .map_err(|err| {
                    println!("Could not add used proofs {:?}", err);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
                .unwrap();

            Ok(Json(split_response))
        }
        Err(err) => {
            println!("Split error: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn post_check_fee(
    Path(_): Path<String>,
    payload: Json<CheckFeesRequest>,
) -> Result<Json<CheckFeesResponse>, StatusCode> {
    // let invoice = LnInvoice::from_str(&payload.pr)?;
    // todo
    let _amount_msat = payload.pr.amount_milli_satoshis().unwrap();
    let amount_sat = 0;
    let amount = Amount::from(amount_sat);

    let fee = amount;

    Ok(Json(CheckFeesResponse { fee }))
}

pub async fn faucet(
    Path(mint_id): Path<String>,
    cashu: Extension<Arc<CashuService<Client>>>,
    params: Query<FaucetQueryParams>,
) -> Result<Json<FaucetResponse>, StatusCode> {
    let token = cashu
        .mint_token(&mint_id, params.amount * 1000)
        .await
        .unwrap();

    Ok(Json(FaucetResponse { token }))
}

pub async fn post_melt(
    Path(mint_id): Path<String>,
    cashu: Extension<Arc<CashuService<Client>>>,
    config: Extension<Arc<CashuApiConfig>>,
    payload: Json<MeltRequest>,
) -> Result<Json<MeltResponse>, StatusCode> {
    let mut mint = cashu.mints.get(&mint_id).unwrap().lock().await;

    if mint.verify_melt_request(&payload).is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let inv = payload.pr.clone();

    let mut node_client =
        LightningNodeGrpcClient::new(config.lightning_node_endpoint.clone(), false)
            .await
            .unwrap();

    info!("Paying invoice");
    let pay_res = node_client
        .pay_invoice(PayInvoiceRequest {
            bolt11_invoice: payload.pr.to_string(),
            amount_msat: inv.amount_milli_satoshis().unwrap_or_default(),
        })
        .await;

    match pay_res {
        Ok(_) => {} //Ok(res.into_inner())},
        Err(_) => {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    info!("Invoice paid");
    let hash = to_string(&inv.payment_hash());
    info!("Hash: {}", hash);
    info!("{:02X?}", inv.payment_hash());
    let total_spent = Amount::from_msat(inv.amount_milli_satoshis().unwrap());

    info!("Invoice amount: {} sats", total_spent.to_sat());
    info!(
        "Proofs amount val: {} sats",
        payload.proofs_amount().to_sat()
    );

    info!("Processing melt request");
    let melt_response = match mint.process_melt_request(&payload, hash.as_str(), total_spent) {
        Ok(res) => res,
        Err(err) => {
            println!("Could not process melt: {}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    info!(
        "Melt response change amount: {}",
        melt_response.change_amount().to_sat()
    );

    info!("Storing used proofs");
    cashu
        .repository
        .add_used_proofs(mint_id.clone(), &payload.proofs)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .unwrap();

    let in_circulation = cashu.repository.get_in_circulation(&mint_id).await.unwrap();
    info!(
        "Tokens in circulation before: {} sats",
        in_circulation.to_sat()
    );
    let in_circulation = in_circulation - (payload.proofs_amount() + melt_response.change_amount());
    info!(
        "Tokens in circulation after: {} sats",
        in_circulation.to_sat()
    );
    cashu
        .repository
        .set_in_circulation(&mint_id, &in_circulation)
        .await
        .unwrap();

    // Process mint request
    Ok(Json(melt_response))
}

//pub async fn post_check(
//    Path(mint_id): Path<String>,
//    cashu: Extension<Arc<CashuService<Client>>>,
//    payload: Json<CheckSpendableRequest>,
//) -> Result<Json<CheckSpendableResponse>, StatusCode> {
//    let mint = cashu.mints.get(&mint_id).unwrap().lock().await;
//
//    let res = mint
//        .(&payload.0)
//        .map_err(|err| {
//            println!("{}", err);
//            StatusCode::INTERNAL_SERVER_ERROR
//        })
//        .unwrap();
//
//    println!("{:?}", res);
//
//    Json(res)
//}

#[inline]
pub fn to_string(value: &[u8]) -> String {
    let mut res = String::with_capacity(2 * value.len());
    for v in value {
        write!(&mut res, "{:02x}", v).expect("Unable to write");
    }
    res
}
