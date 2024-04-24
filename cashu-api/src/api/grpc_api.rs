use std::sync::Arc;

use cashu_grpc_api::*;
use database::surrealdb;
use surrealdb::engine::remote::ws::Client;
use tonic::{Request, Response, Status};

use crate::cashu::CashuService;

pub mod cashu_grpc_api {
    tonic::include_proto!("cashu_grpc_service");
}

pub struct CashuGrpcService {
    pub cashu_service: Arc<CashuService<Client>>,
}

#[tonic::async_trait]
impl cashu_server::Cashu for CashuGrpcService {
    async fn internal_token_mint(
        &self,
        request: Request<InternalTokenMintRequest>,
    ) -> Result<Response<InternalTokenMintResponse>, Status> {
        let r = request.into_inner();
        let token = self
            .cashu_service
            .mint_token(&r.mint_id, r.amount_sat)
            .await
            .unwrap();

        Ok(Response::new(InternalTokenMintResponse { token }))
    }

    async fn create_mint(
        &self,
        request: Request<CreateMintRequest>,
    ) -> Result<Response<CreateMintResponse>, Status> {
        let r = request.into_inner();

        match self
            .cashu_service
            .new_mint(
                r.name.as_str(),
                r.version.as_str(),
                r.secret.as_str(),
                r.derivation_path.as_str(),
                r.max_order.try_into().unwrap(),
                r.min_fee_reserve_msat,
                r.percent_fee_reserve,
                Some(r.description),
                Some(r.description_long),
                Some(r.contact),
                Some(r.motd)
            )
            .await
        {
            Ok(_) => Ok(Response::new(CreateMintResponse {})),
            Err(err) => Err(Status::new(tonic::Code::Internal, err.to_string())),
        }
    }
}
