use crate::domain::{
    Brand, Category, Generation, Money, Product, ProductAttribute, ProductFitment, ProductPage,
    SaleType, Timeframe, Trim, VehicleConfiguration, VehicleModel,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicProductResponse {
    pub id: String,
    pub sku: String,
    pub model_number: Option<String>,
    pub brand_id: String,
    pub price: Money,
    pub name: String,
    pub attributes: Vec<ProductAttribute>,
    pub description: Option<String>,
    pub image_name: String,
    pub image_urls: Vec<String>,
    pub sale_type: SaleType,
    pub category_ids: Vec<String>,
}

impl From<Product> for PublicProductResponse {
    fn from(value: Product) -> Self {
        Self {
            id: value.id.0,
            sku: value.sku,
            model_number: value.model_number,
            brand_id: value.brand_id.0,
            price: value.price,
            name: value.name,
            attributes: value.attributes,
            description: value.description,
            image_name: value.image_name,
            image_urls: value.image_urls,
            sale_type: value.sale_type,
            category_ids: value.category_ids.into_iter().map(|id| id.0).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicProductPageResponse {
    pub items: Vec<PublicProductResponse>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

impl From<ProductPage> for PublicProductPageResponse {
    fn from(value: ProductPage) -> Self {
        let has_more = value.next_cursor.is_some();
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            next_cursor: value.next_cursor,
            has_more,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleModelCreatedResponse {
    pub id: String,
    pub name: String,
    pub model_code: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleModelOptionResponse {
    pub id: String,
    pub name: String,
    pub model_code: String,
}

impl From<VehicleModel> for VehicleModelOptionResponse {
    fn from(value: VehicleModel) -> Self {
        Self {
            id: value.id.0,
            name: value.name,
            model_code: value.model_code,
        }
    }
}

impl From<VehicleModel> for VehicleModelCreatedResponse {
    fn from(value: VehicleModel) -> Self {
        Self {
            id: value.id.0,
            name: value.name,
            model_code: value.model_code,
            created_at: value.metadata.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationCreatedResponse {
    pub id: String,
    pub vehicle_model_id: String,
    pub name: String,
    pub generation_code: String,
    pub timeframe: Timeframe,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<Generation> for GenerationCreatedResponse {
    fn from(value: Generation) -> Self {
        Self {
            id: value.id.0,
            vehicle_model_id: value.vehicle_model_id.0,
            name: value.name,
            generation_code: value.generation_code,
            timeframe: value.timeframe,
            created_at: value.metadata.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationOptionResponse {
    pub id: String,
    pub name: String,
    pub code: String,
    pub vehicle_model_id: String,
}
impl From<Generation> for GenerationOptionResponse {
    fn from(value: Generation) -> Self {
        Self {
            id: value.id.0,
            name: value.name,
            code: value.generation_code,
            vehicle_model_id: value.vehicle_model_id.0,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedOptionResponse {
    pub id: String,
    pub name: String,
    pub code: String,
}
impl From<Brand> for NamedOptionResponse {
    fn from(value: Brand) -> Self {
        Self {
            id: value.id.0,
            name: value.name,
            code: value.brand_code,
        }
    }
}
impl From<Category> for NamedOptionResponse {
    fn from(value: Category) -> Self {
        Self {
            id: value.id.0,
            name: value.name,
            code: value.category_code,
        }
    }
}
impl From<Generation> for NamedOptionResponse {
    fn from(value: Generation) -> Self {
        Self {
            id: value.id.0,
            name: value.name,
            code: value.generation_code,
        }
    }
}
impl From<Trim> for NamedOptionResponse {
    fn from(value: Trim) -> Self {
        Self {
            id: value.id.0,
            name: value.name,
            code: value.trim_code,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrimOptionResponse {
    pub id: String,
    pub name: String,
    pub code: String,
    pub generation_id: String,
}
impl From<Trim> for TrimOptionResponse {
    fn from(value: Trim) -> Self {
        Self {
            id: value.id.0,
            name: value.name,
            code: value.trim_code,
            generation_id: value.generation_id.0,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineCreatedResponse {
    pub id: String,
    pub vehicle_model: String,
    pub vehicle_generation: String,
    pub vehicle_trims: Vec<String>,
    pub alloy_material: String,
    pub factory_code: String,
    pub displacement: f64,
    pub horsepower: i32,
    pub rpm: i32,
    pub layout: crate::domain::EngineLayout,
    pub aspiration_type: crate::domain::AspirationType,
    pub fuel_delivery: crate::domain::FuelDeliveryType,
}
impl From<crate::domain::Engine> for EngineCreatedResponse {
    fn from(value: crate::domain::Engine) -> Self {
        Self {
            id: value.id.0,
            vehicle_model: value.vehicle_model.0,
            vehicle_generation: value.vehicle_generation.0,
            vehicle_trims: value.vehicle_trims.into_iter().map(|id| id.0).collect(),
            alloy_material: value.alloy_material,
            factory_code: value.factory_code,
            displacement: value.displacement,
            horsepower: value.horsepower,
            rpm: value.rpm,
            layout: value.layout,
            aspiration_type: value.aspiration_type,
            fuel_delivery: value.fuel_delivery,
        }
    }
}
impl From<Product> for NamedOptionResponse {
    fn from(value: Product) -> Self {
        Self {
            id: value.id.0,
            name: value.name,
            code: value.sku,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCreatedResponse {
    pub id: String,
    pub category_code: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
impl From<Category> for CategoryCreatedResponse {
    fn from(value: Category) -> Self {
        Self {
            id: value.id.0,
            category_code: value.category_code,
            parent_id: value.parent_id.map(|id| id.0),
            name: value.name,
            description: value.description,
            created_at: value.metadata.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrimCreatedResponse {
    pub id: String,
    pub generation_id: String,
    pub vehicle_models: Vec<String>,
    pub name: String,
    pub trim_code: String,
    pub timeframe: Timeframe,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
impl From<Trim> for TrimCreatedResponse {
    fn from(value: Trim) -> Self {
        Self {
            id: value.id.0,
            generation_id: value.generation_id.0,
            vehicle_models: value.vehicle_models.into_iter().map(|id| id.0).collect(),
            name: value.name,
            trim_code: value.trim_code,
            timeframe: value.timeframe,
            created_at: value.metadata.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandCreatedResponse {
    pub id: String,
    pub brand_code: String,
    pub name: String,
    pub image_name: String,
    pub description: Option<String>,
}
impl From<Brand> for BrandCreatedResponse {
    fn from(v: Brand) -> Self {
        Self {
            id: v.id.0,
            brand_code: v.brand_code,
            name: v.name,
            image_name: v.image_name,
            description: v.description,
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleConfigurationCreatedResponse {
    pub id: String,
    pub trim_id: String,
    pub model_year: u16,
}
impl From<VehicleConfiguration> for VehicleConfigurationCreatedResponse {
    fn from(v: VehicleConfiguration) -> Self {
        Self {
            id: v.id.0,
            trim_id: v.trim_id.0,
            model_year: v.model_year,
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductFitmentCreatedResponse {
    pub id: String,
    pub product_id: String,
    pub system: crate::domain::VehicleSystem,
    pub component: crate::domain::ComponentType,
}
impl From<ProductFitment> for ProductFitmentCreatedResponse {
    fn from(v: ProductFitment) -> Self {
        Self {
            id: v.id.0,
            product_id: v.product_id.0,
            system: v.system,
            component: v.component,
        }
    }
}
