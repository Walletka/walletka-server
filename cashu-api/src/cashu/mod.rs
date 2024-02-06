use anyhow::{bail, Result};
use cashu_sdk::{
    mint::Mint,
    nuts::{BlindedMessages, MintRequest, Token},
    types::InvoiceStatus,
    url::UncheckedUrl,
    Amount,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use database::surrealdb::Connection;
use tokio::sync::Mutex;

use crate::{repositories::cashu_repository::CashuMintReporitory, types::StoredMint};

use self::utils::unix_time;

//pub mod database;
pub mod error;
pub mod utils;

#[derive(Clone)]
pub struct CashuService<C>
where
    C: Connection,
{
    pub repository: Arc<CashuMintReporitory<C>>,
    pub mints: HashMap<String, Arc<Mutex<Mint>>>,
    pub mint_url: String,
}

impl<C> CashuService<C>
where
    C: Connection,
{
    pub async fn init(
        mint_url: String,
        cashu_mint_repository: Arc<CashuMintReporitory<C>>,
    ) -> Result<Self, anyhow::Error> {
        let mints = cashu_mint_repository.get_all_mints().await?;
        let mints = mints
            .into_iter()
            .map(|m| (m.name.clone(), Arc::new(Mutex::new(m.into()))))
            .collect();
        let cashu_service = CashuService {
            repository: cashu_mint_repository,
            mints,
            mint_url,
        };

        Ok(cashu_service)
    }

    pub async fn new_mint(
        &self,
        name: &str,
        version: &str,
        secret: &str,
        derivation_path: &str,
        max_order: u8,
        min_fee_reserve_msat: u64,
        percent_fee_reserve: f32,
        description: Option<String>,
        description_long: Option<String>,
        contact: Option<HashMap<String, String>>,
        motd: Option<String>,
    ) -> Result<()> {
        if self.repository.get_mint(name.to_string()).await.is_ok() {
            bail!("Mint with same name already exists!")
        }

        let mint = Mint::new(
            secret,
            derivation_path,
            HashSet::default(),
            HashSet::default(),
            max_order,
            Amount::from_msat(min_fee_reserve_msat),
            percent_fee_reserve,
        );

        let stored_mint = StoredMint {
            name: name.to_string(),
            version: Some(version.to_string()),
            description,
            description_long,
            contact,
            nuts: None,
            motd,
            secret: Some(secret.to_string()),
            derivation_path: Some(derivation_path.to_string()),
            active_keyset: Some(mint.active_keyset.id.to_string()),
            keysets_info: Some(mint.inactive_keysets.into_iter().map(|k| k.1).collect()),
            in_circulation_msat: 0,
            spend_secrets: None,
            max_order: Some(max_order),
            min_fee_reserve_msat: Some(min_fee_reserve_msat),
            percent_fee_reserve: Some(percent_fee_reserve),
        };

        self.repository.add_mint(stored_mint.clone()).await?;

        // let mint: Arc<Mutex<Mint>> = Arc::new(Mutex::new(stored_mint.into()));
        //todo: self.mints.insert(name.to_string(), mint);

        Ok(())
    }

    pub async fn handle_paid_invoice(
        &self,
        payment_hash: cashu_sdk::Sha256,
    ) -> Result<(), anyhow::Error> {
        let mut invoice_info = self
            .repository
            .get_invoice_info_by_payment_hash(&payment_hash)
            .await?;

        invoice_info.status = InvoiceStatus::Paid;
        invoice_info.confirmed_at = Some(unix_time());

        self.repository.add_invoice(&invoice_info).await?;

        Ok(())
    }

    pub async fn mint_token(&self, mint_id: &str, amount_msat: u64) -> Result<String> {
        let blinded_messages = BlindedMessages::random(Amount::from_msat(amount_msat))?;

        let mut mint = self
            .mints
            .get(mint_id)
            .expect("Mint not exists!")
            .lock()
            .await;
        let keys = mint.active_keyset_pubkeys();

        let proofs = match mint.process_mint_request(MintRequest {
            outputs: blinded_messages.blinded_messages,
        }) {
            Ok(res) => {
                let proofs = cashu_sdk::dhke::construct_proofs(
                    res.promises,
                    blinded_messages.rs,
                    blinded_messages.secrets,
                    &cashu_sdk::nuts::nut01::Keys::new(keys.keys.keys()),
                )?;
                proofs
            }
            Err(_) => bail!("Cannot get proofs"),
        };

        let in_circulation = self.repository.get_in_circulation(mint_id).await.unwrap()
            + Amount::from_msat(amount_msat);

        self.repository
            .set_in_circulation(mint_id, &in_circulation)
            .await
            .ok();

        let token = Token::new(
            UncheckedUrl::new(format!("{}/{}", self.mint_url.clone(), mint_id)),
            proofs,
            None,
        )
        .unwrap();

        dbg!("Generated token: {}", token.convert_to_string().unwrap());

        Ok(token.convert_to_string().unwrap())
    }
}
