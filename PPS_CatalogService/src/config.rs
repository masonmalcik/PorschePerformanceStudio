use crate::AppError;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub mongodb_uri: String,
    pub database_name: String,
    pub asset_root: PathBuf,
    pub admin_auth_mode: String,
    pub admin_development_token: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            mongodb_uri: required("MONGODB_URI")?,
            database_name: optional("MONGODB_DATABASE", "pps_catalog"),
            asset_root: PathBuf::from(optional("ASSET_ROOT", "assets")),
            admin_auth_mode: optional("ADMIN_AUTH_MODE", "disabled"),
            admin_development_token: std::env::var("ADMIN_DEV_TOKEN").ok(),
        })
    }
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
