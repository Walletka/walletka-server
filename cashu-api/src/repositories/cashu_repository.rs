use std::collections::HashSet;

use anyhow::{bail, Result};
use cashu_sdk::{
    nuts::{Proof, Proofs},
    secret::Secret,
    Amount, Sha256,
};
use serde::{Deserialize, Serialize};
use database::surrealdb::{sql::Id, Connection, Surreal};

use crate::types::{InvoiceInfo, StoredMint, UsedProof};


#[derive(Serialize, Deserialize)]
struct MintIdWrapper<T> {
    pub mint_id: String,
    pub id: T,
}

pub struct CashuMintReporitory<C>
where
    C: Connection,
{
    db: Surreal<C>,
}

impl<C> CashuMintReporitory<C>
where
    C: Connection,
{
    pub fn new(db: Surreal<C>) -> Self {
        Self { db }
    }

    pub async fn add_mint(&self, mint_info: StoredMint) -> Result<()> {
        let created_mint_info: Option<StoredMint> = self
            .db
            .create(("mint", mint_info.name.clone()))
            .content(mint_info)
            .await?;

        dbg!(created_mint_info);

        Ok(())
    }

    pub async fn get_mint(&self, name: String) -> Result<StoredMint> {
        let res: Option<StoredMint> = match self
            .db
            .query(format!(
                "SELECT *, name as name, 
                (SELECT proof.secret as secret FROM used_proof WHERE mint_id = $parent.name).secret as spend_secrets 
                FROM ONLY mint:{}",
                name
            ))
            .await?
            .take(0) {
                Ok(mint_info) => mint_info,
                Err(_) => bail!("Mint not found!"),
            };

        match res {
            Some(mint_info) => Ok(mint_info),
            None => bail!("Mint not found"),
        }
    }

    pub async fn get_all_mints(&self) -> Result<Vec<StoredMint>> {
        let res: Vec<StoredMint> = self.db
        .query(
            "SELECT *, (SELECT proof.secret as secret FROM used_proof WHERE mint_id = $parent.name).secret as spend_secrets FROM mint"
        ).await?.take(0).unwrap();

        Ok(res)
    }

    pub async fn set_active_keyset(&self, mint_id: String, keyset_id: &Id) -> Result<()> {
        let _ = self
            .db
            .query(format!(
                "UPDATE mint:{} SET active_mint = {}",
                mint_id,
                keyset_id.to_string()
            ))
            .await?;

        Ok(())
    }

    pub async fn add_invoice(&self, invoice_info: &InvoiceInfo) -> Result<()> {
        let added_invoice: Option<InvoiceInfo> = self
            .db
            .create(("invoice", invoice_info.hash.to_string()))
            .content(invoice_info)
            .await?;

        dbg!(added_invoice);

        Ok(())
    }

    pub async fn get_invoice_info(&self, hash: &Sha256) -> Result<InvoiceInfo> {
        let invoice_info: Option<InvoiceInfo> =
            self.db.select(("invoice", hash.to_string())).await?;

        match invoice_info {
            Some(invoice_info) => Ok(invoice_info),
            None => bail!("Invoice not found"),
        }
    }

    pub async fn get_invoice_info_by_payment_hash(
        &self,
        payment_hash: &Sha256,
    ) -> Result<InvoiceInfo> {
        let mut res = self
            .db
            .query("SELECT * FROM ONLY invoice WHERE payment_hash = ($payment_hash)")
            .bind(("payment_hash", payment_hash.to_string()))
            .await?;

        let res: Option<InvoiceInfo> = res.take(0).unwrap();

        match res {
            Some(invoice_info) => Ok(invoice_info),
            None => bail!("Invoice not found"),
        }
    }

    pub async fn add_used_proofs(&self, mint_id: String, proofs: &Proofs) -> Result<()> {
        let used_proofs: Vec<UsedProof> = proofs
            .into_iter()
            .map(|p| UsedProof {
                mint_id: mint_id.clone(),
                proof: p.clone(),
            })
            .collect();

        for proof in used_proofs {
            let _: Vec<Option<UsedProof>> = self.db.create("used_proof").content(proof).await?;
        }

        Ok(())
    }

    pub async fn get_spent_secrets(&self, mint_id: String) -> Result<HashSet<Secret>> {
        let used_proofs: Vec<Proof> = self
            .db
            .query(format!(
                "SELECT * FROM used_proof WHERE mint_id = {}",
                mint_id
            ))
            .await?
            .take(0)
            .unwrap();

        Ok(used_proofs.into_iter().map(|p| p.secret).collect())
    }
    pub async fn get_in_circulation(&self, mint_id: &str) -> Result<Amount> {
        let res: Option<StoredMint> = self
            .db
            .query(format!(
                "SELECT name, in_circulation_msat FROM mint:{}",
                mint_id
            ))
            .await?
            .take(0)
            .unwrap();

        match res {
            Some(res) => Ok(Amount::from_msat(res.in_circulation_msat)),
            None => bail!("Mint not found!"),
        }
    }

    pub async fn set_in_circulation(&self, mint_id: &str, amount: &Amount) -> Result<()> {
        let _ = self
            .db
            .query(format!(
                "UPDATE mint:{} SET in_circulation_msat = {}",
                mint_id,
                amount.to_msat()
            ))
            .await?;

        Ok(())
    }
}
