mod data;

use crate::{data::mongodb::migrations::migrate, AppError};
use mongodb::{
    bson::{doc, oid::ObjectId, DateTime, Decimal128, Document},
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
    pub products: usize,
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

    let asset_base_url = std::env::var("ASSET_BASE_URL")
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_owned();
    let products = database.collection::<Document>("products");
    for seed in data::PRODUCTS {
        let brand_id = brands
            .find_one(doc! { "brandCode": seed.brand_code })
            .await?
            .and_then(|value| value.get_object_id("_id").ok())
            .ok_or_else(|| {
                AppError::InvalidData(format!("missing seed brand {}", seed.brand_code))
            })?;
        let category_id = categories
            .find_one(doc! { "categoryCode": seed.category_code })
            .await?
            .and_then(|value| value.get_object_id("_id").ok())
            .ok_or_else(|| {
                AppError::InvalidData(format!("missing seed category {}", seed.category_code))
            })?;
        let price = Decimal128::from_str(seed.price)
            .map_err(|error| AppError::InvalidData(format!("invalid seed price: {error}")))?;
        let image_urls = if asset_base_url.is_empty() {
            Vec::<String>::new()
        } else {
            vec![format!("{asset_base_url}/products/{}", seed.image_name)]
        };
        products.update_one(doc! { "sku": seed.sku }, doc! {
            "$set": {
                "modelNumber": seed.model_number,
                "brandId": brand_id,
                "price": price,
                "currency": "USD",
                "name": seed.name,
                "attributes": [],
                "description": seed.description,
                "imageName": seed.image_name,
                "imageUrls": image_urls,
                "saleType": "retail",
                "categoryIds": [category_id],
                "updatedAt": DateTime::now(),
                "isActive": true
            },
            "$setOnInsert": { "_id": ObjectId::new(), "createdAt": DateTime::now(), "version": 1_i64 }
        }).upsert(true).await?;
    }

    database.collection::<Document>("_catalog_seed_runs").insert_one(doc! {
        "profile": format!("{profile:?}").to_ascii_lowercase(), "version": 1_i32,
        "checksum": "reference-taxonomy-and-products-v3", "executedAt": DateTime::now(), "status": "succeeded"
    }).await?;
    Ok(SeedSummary {
        brands: data::BRANDS.len(),
        categories: data::CATEGORIES.len(),
        products: data::PRODUCTS.len(),
    })
}
