use anyhow::{Error, Ok, Result};

use database::surrealdb::{
    sql::{Id, Thing},
    Connection, Surreal,
};

use crate::entity::LspInvoice;

pub struct LspInvoiceRepository<C>
where
    C: Connection,
{
    db: Surreal<C>,
}

impl<C> LspInvoiceRepository<C>
where
    C: Connection,
{
    pub fn new(db: Surreal<C>) -> Self {
        Self { db }
    }

    pub async fn add_invoice(
        &self,
        lsp_invoice: LspInvoice,
        lsp_customer_alias: String,
    ) -> Result<LspInvoice> {
        let mut lsp_invoice = lsp_invoice;
        lsp_invoice.id = Some(Thing {
            tb: "invoice".to_string(),
            id: Id::String(lsp_invoice.payment_hash.clone()),
        });

        let created: Vec<LspInvoice> = self.db.create("invoice").content(lsp_invoice).await?;
        let created = match created.into_iter().next() {
            Some(c) => c,
            None => return Err(Error::msg("Can't store new invoice")),
        };

        self.db
            .query(format!("RELATE invoice:{} -> issued_for -> customer:{}", created.payment_hash, lsp_customer_alias))
            .await?;

        Ok(created)
    }
}
