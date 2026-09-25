use pps_catalog_service::{
    data::mongodb::connection::connect,
    seed::{run, SeedProfile},
    Config,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let profile = std::env::args()
        .skip_while(|value| value != "--profile")
        .nth(1)
        .unwrap_or_else(|| "minimal".into())
        .parse::<SeedProfile>()?;
    let config = Config::from_env()?;
    let database = connect(&config).await?;
    let summary = run(&database, profile).await?;
    println!(
        "Seed complete: {} brands, {} categories",
        summary.brands, summary.categories
    );
    Ok(())
}
