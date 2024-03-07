use anyhow::{bail, Error, Result};
use serde::Deserialize;
use database::surrealdb::{
    sql::{Id, Thing},
    Connection, Surreal,
};

use crate::entity::{LspCustomer, LspCustomerConfig};

pub struct LspCustomerRepository<C>
where
    C: Connection,
{
    db: Surreal<C>,
}

impl<C> LspCustomerRepository<C>
where
    C: Connection,
{
    pub fn new(db: Surreal<C>) -> Self {
        Self { db }
    }
    pub async fn add_customer(&self, lsp_customer: LspCustomer) -> Result<LspCustomer> {
        let mut lsp_customer = lsp_customer;
        lsp_customer.id = Some(Thing {
            tb: "customers".to_string(),
            id: Id::String(lsp_customer.alias.clone()),
        });

        let created: Vec<LspCustomer> = self.db.create("customer").content(lsp_customer).await?;

        match created.into_iter().next() {
            Some(c) => Ok(c),
            None => Err(Error::msg("Can't create new customer")),
        }
    }

    pub async fn update_customer_lsp_config(
        &self,
        alias: &str,
        lsp_customer_config: LspCustomerConfig,
    ) -> Result<LspCustomerConfig> {
        let mut res = self
            .db
            .query(format!(
                "UPDATE ONLY customer SET config = ($config) WHERE alias = {} LIMIT 1",
                alias
            ))
            .bind(("config", lsp_customer_config))
            .await?;

        if res.num_statements() == 0 {
            bail!("Lsp customer config cannot be updated!")
        }

        let customer: Option<LspCustomer> = res.take(0).unwrap();
        match customer {
            Some(customer) => return Ok(customer.config),
            None => bail!("Lsp custommer cannot be updated!"),
        }
    }

    pub async fn get_customers(&self) -> Result<Vec<LspCustomer>> {
        let customers: Vec<LspCustomer> = self.db.select("customer").await?;

        Ok(customers)
    }

    pub async fn get_customer_by_alias(&self, alias: String) -> Result<Option<LspCustomer>> {
        let res = self.db.select(("customer", alias)).await?;

        Ok(res)
    }

    pub async fn get_customer_by_npub(&self, npub: String) -> Result<Option<LspCustomer>> {
        let mut res = self
            .db
            .query("SELECT * FROM ONLY customer WHERE npub == ($npub) LIMIT 1")
            .bind(("npub", npub))
            .await?;

        match res.take(0) {
            Ok(customer) => Ok(customer),
            Err(_) => Ok(None),
        }
    }

    pub async fn get_customer_by_payment_hash(
        &self,
        payment_hash: String,
    ) -> Result<Option<LspCustomer>> {
        #[derive(Deserialize)]
        struct Wrapper {
            root: Option<LspCustomer>,
        }

        let mut res = self
            .db
            .query(format!(
                "SELECT (SELECT * FROM ONLY ->issued_for->customer LIMIT 1) as root FROM invoice:{}",
                payment_hash
            ))
            .await?;

        let wrapper: Result<Option<Wrapper>, database::surrealdb::Error> = res.take(0);

        match wrapper {
            Ok(wrapper) => match wrapper {
                Some(wrapper) => Ok(wrapper.root),
                None => Ok(None),
            },
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use database::surrealdb::engine::local::Mem;
    use database::surrealdb::Surreal;

    use crate::entity::{LspCustomer, LspCustomerConfig};

    use super::LspCustomerRepository;

    #[tokio::test]
    async fn test_create_user() {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("test").await.unwrap();
        db.use_db("test").await.unwrap();

        let repository = LspCustomerRepository { db };

        let customer = LspCustomer {
            id: None,
            node_id: Some("fake node id".to_string()),
            npub: Some("None".to_string()),
            alias: "fake alias".to_string(),
            config: LspCustomerConfig::default(),
        };

        let created_customer = repository.add_customer(customer).await;
        let by_npub = repository.get_customer_by_npub("None".to_string()).await;
        assert!(by_npub.is_ok() == true);
        assert!(by_npub.unwrap().is_some());
        assert!(created_customer.is_ok());
        assert!(created_customer.unwrap().id.is_some());
    }
}
