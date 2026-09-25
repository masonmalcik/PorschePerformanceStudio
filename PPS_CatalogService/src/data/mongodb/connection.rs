use crate::{AppError, Config};
use mongodb::{options::ClientOptions, Client, Database};

pub async fn connect(config: &Config) -> Result<Database, AppError> {
    let options = ClientOptions::parse(&config.mongodb_uri).await?;
    let client = Client::with_options(options)?;
    Ok(client.database(&config.database_name))
}
