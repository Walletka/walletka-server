use serde::Deserialize;

#[derive(Deserialize)]
pub struct SurrealDbConfig {
    pub db_endpoint: String,
    pub db_user: String,
    pub db_pass: String,
}
