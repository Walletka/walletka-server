use std::{collections::HashMap, sync::Arc};

use axum::{extract::Query, http::StatusCode, Extension};
use database::surrealdb::engine::remote::ws::Client;

use crate::services::lsp_customer_service::LspCustomerService;

use super::models::Nip05Params;

pub async fn nip05(
    lsp_customer_service: Extension<Arc<LspCustomerService<Client>>>,
    Query(params): Query<Nip05Params>,
) -> Result<String, StatusCode> {
    match params.name {
        Some(name) => {
            let user = lsp_customer_service
                .get_customer_by_alias(name.clone())
                .await;

            match user {
                Some(user) => match user.npub {
                    Some(npub) => {
                        let response = format!(
                            r#"{{ names {{ {}: {} }} }}"#,
                            name, npub
                        );

                        Ok(response)
                    }
                    None => Err(StatusCode::NOT_FOUND),
                },
                None => Err(StatusCode::NOT_FOUND),
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}
