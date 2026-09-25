use pps_catalog_service::{
    data::mongodb::{connection::connect, migrations::migrate},
    Config,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    let database = connect(&config).await?;
    migrate(&database).await?;
    println!("Catalog database migrations are current.");
    Ok(())
}
