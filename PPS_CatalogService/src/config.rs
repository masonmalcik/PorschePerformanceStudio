use crate::AppError;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub mongodb_uri: String,
    pub database_name: String,
    pub asset_root: PathBuf,
    pub admin_auth_mode: String,
    pub admin_development_token: Option<String>,
    pub cognito_issuer: Option<String>,
    pub cognito_client_id: Option<String>,
    pub cognito_jwks_url: Option<String>,
    pub redis_mode: String,
    pub redis_cluster_urls: Vec<String>,
    pub redis_pool_size: u32,
    pub elasticsearch_url: Option<String>,
    pub elasticsearch_timeout_ms: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            mongodb_uri: required("MONGODB_URI")?,
            database_name: optional("MONGODB_DATABASE", "pps_catalog"),
            asset_root: PathBuf::from(optional("ASSET_ROOT", "assets")),
            admin_auth_mode: optional("ADMIN_AUTH_MODE", "disabled"),
            admin_development_token: std::env::var("ADMIN_DEV_TOKEN").ok(),
            cognito_issuer: nonempty("COGNITO_ISSUER"),
            cognito_client_id: nonempty("COGNITO_CLIENT_ID"),
            cognito_jwks_url: nonempty("COGNITO_JWKS_URL"),
            redis_mode: optional("REDIS_MODE", "cluster").to_lowercase(),
            redis_cluster_urls: std::env::var("REDIS_CLUSTER_URLS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect(),
            redis_pool_size: positive_u32("REDIS_POOL_SIZE", 16)?,
            elasticsearch_url: std::env::var("ELASTICSEARCH_URL")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            elasticsearch_timeout_ms: u64::from(positive_u32("ELASTICSEARCH_TIMEOUT_MS", 500)?),
        })
    }
}

fn nonempty(name: &'static str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn required(name: &'static str) -> Result<String, AppError> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or(AppError::Configuration(format!("{name} must be set")))
}

fn optional(name: &'static str, default: &'static str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn positive_u32(name: &'static str, default: u32) -> Result<u32, AppError> {
    let value = std::env::var(name)
        .ok()
        .map(|raw| raw.parse::<u32>())
        .transpose()
        .map_err(|_| AppError::Configuration(format!("{name} must be a positive integer")))?
        .unwrap_or(default);
    if value == 0 {
        return Err(AppError::Configuration(format!(
            "{name} must be a positive integer"
        )));
    }
    Ok(value)
}
