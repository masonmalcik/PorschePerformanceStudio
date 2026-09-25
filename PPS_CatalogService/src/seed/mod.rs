mod data;

use crate::{data::mongodb::migrations::migrate, AppError};
use mongodb::{
    bson::{doc, oid::ObjectId, DateTime, Document},
    Database,
};
use serde::Serialize;
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug)]
pub enum SeedProfile {
    Development,
    Minimal,
    Production,
}

impl FromStr for SeedProfile {
    type Err = SeedProfileError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "development" => Ok(Self::Development),
            "minimal" => Ok(Self::Minimal),
            "production" => Ok(Self::Production),
            _ => Err(SeedProfileError(value.into())),
        }
    }
}

#[derive(Debug)]
pub struct SeedProfileError(String);
impl fmt::Display for SeedProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown seed profile: {}", self.0)
    }
}
impl std::error::Error for SeedProfileError {}

#[derive(Debug, Serialize)]
pub struct SeedSummary {
    pub brands: usize,
    pub categories: usize,
}

pub async fn run(database: &Database, profile: SeedProfile) -> Result<SeedSummary, AppError> {
    migrate(database).await?;
    let brands = database.collection::<Document>("brands");
    for seed in data::BRANDS {
        brands.update_one(doc! { "brandCode": seed.code }, doc! {
            "$set": { "name": seed.name, "imageName": seed.image_name, "description": seed.description, "updatedAt": DateTime::now(), "isActive": true },
            "$setOnInsert": { "_id": ObjectId::new(), "createdAt": DateTime::now(), "version": 1_i64 }
        }).upsert(true).await?;
    }

    let categories = database.collection::<Document>("categories");
    for seed in data::CATEGORIES {
        let parent_id = match seed.parent_code {
            Some(code) => categories
                .find_one(doc! { "categoryCode": code })
                .await?
                .and_then(|d| d.get_object_id("_id").ok()),
            None => None,
        };
        categories.update_one(doc! { "categoryCode": seed.code }, doc! {
            "$set": { "name": seed.name, "description": seed.description, "parentId": parent_id, "attributes": [], "updatedAt": DateTime::now(), "isActive": true },
            "$setOnInsert": { "_id": ObjectId::new(), "createdAt": DateTime::now(), "version": 1_i64 }
        }).upsert(true).await?;
    }

    database.collection::<Document>("_catalog_seed_runs").insert_one(doc! {
        "profile": format!("{profile:?}").to_ascii_lowercase(), "version": 1_i32,
        "checksum": "reference-taxonomy-v2", "executedAt": DateTime::now(), "status": "succeeded"
    }).await?;
    Ok(SeedSummary {
        brands: data::BRANDS.len(),
        categories: data::CATEGORIES.len(),
    })
}
