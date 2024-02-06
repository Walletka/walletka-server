use anyhow::Result;
use config::SurrealDbConfig;
use surrealdb::{
    engine::remote::ws::{Client, Wss},
    opt::auth::Root,
    Surreal,
};

pub use surrealdb;

pub mod config;

pub async fn init_db(
    config: SurrealDbConfig,
    namespace: &str,
    database: &str,
) -> Result<Surreal<Client>> {
    let db: Surreal<Client> = Surreal::init();

    db.connect::<Wss>(config.db_endpoint).await?;

    // Sign in if credentials are provided
    db.signin(Root {
        username: &config.db_user,
        password: &config.db_pass,
    })
    .await
    .expect("Cannt connect to db");

    // Select a namespace + database
    db.use_ns(namespace).use_db(database).await?;

    Ok(db)
}
