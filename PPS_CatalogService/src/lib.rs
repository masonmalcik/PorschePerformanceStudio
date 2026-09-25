#![recursion_limit = "256"]

pub mod application;
pub mod config;
pub mod data;
pub mod domain;
pub mod error;
pub mod presentation;
pub mod seed;

pub use config::Config;
pub use error::AppError;
