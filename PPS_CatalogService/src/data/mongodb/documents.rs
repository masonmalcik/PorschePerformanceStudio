use crate::{domain::*, AppError};
use mongodb::bson::{oid::ObjectId, DateTime, Decimal128};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub sku: String,
    pub model_number: Option<String>,
    pub brand_id: ObjectId,
    pub price: Decimal128,
    pub currency: String,
    pub name: String,
    pub attributes: Vec<ProductAttribute>,
    pub description: Option<String>,
    pub image_name: String,
    pub image_urls: Vec<String>,
    pub sale_type: SaleType,
    pub category_ids: Vec<ObjectId>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub is_active: bool,
    pub version: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleModelDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub name: String,
    pub model_code: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub is_active: bool,
    pub version: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub vehicle_model_id: ObjectId,
    pub name: String,
    pub generation_code: String,
    pub timeframe: Timeframe,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub is_active: bool,
    pub version: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub brand_code: String,
    pub name: String,
    pub image_name: String,
    pub description: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub is_active: bool,
    pub version: u64,
}
impl BrandDocument {
    pub fn from_create(input: CreateBrand) -> Self {
        let now = DateTime::now();
        Self {
            id: ObjectId::new(),
            brand_code: input.brand_code,
            name: input.name,
            image_name: input.image_name,
            description: input.description,
            created_at: now,
            updated_at: now,
            is_active: true,
            version: 1,
        }
    }
    pub fn into_domain(self) -> Result<Brand, AppError> {
        Ok(Brand {
            id: BrandId(self.id.to_hex()),
            brand_code: self.brand_code,
            name: self.name,
            image_name: self.image_name,
            description: self.description,
            metadata: metadata(
                self.created_at,
                self.updated_at,
                self.is_active,
                self.version,
            )?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleConfigurationDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub trim_id: ObjectId,
    pub model_year: u16,
    pub engine: Option<EngineConfiguration>,
    pub transmission: Option<TransmissionConfiguration>,
    pub drivetrain: Option<DrivetrainType>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub is_active: bool,
    pub version: u64,
}
impl VehicleConfigurationDocument {
    pub fn from_create(input: CreateVehicleConfiguration) -> Result<Self, AppError> {
        let now = DateTime::now();
        Ok(Self {
            id: ObjectId::new(),
            trim_id: object_id(&input.trim_id.0)?,
            model_year: input.model_year,
            engine: input.engine,
            transmission: input.transmission,
            drivetrain: input.drivetrain,
            created_at: now,
            updated_at: now,
            is_active: true,
            version: 1,
        })
    }
    pub fn into_domain(self) -> Result<VehicleConfiguration, AppError> {
        Ok(VehicleConfiguration {
            id: VehicleConfigurationId(self.id.to_hex()),
            trim_id: TrimId(self.trim_id.to_hex()),
            model_year: self.model_year,
            engine: self.engine,
            transmission: self.transmission,
            drivetrain: self.drivetrain,
            metadata: metadata(
                self.created_at,
                self.updated_at,
                self.is_active,
                self.version,
            )?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub vehicle_model: ObjectId,
    pub vehicle_generation: ObjectId,
    pub vehicle_trims: Vec<ObjectId>,
    pub alloy_material: String,
    pub factory_code: String,
    pub displacement: f64,
    pub horsepower: i32,
    pub rpm: i32,
    pub layout: EngineLayout,
    pub aspiration_type: AspirationType,
    pub fuel_delivery: FuelDeliveryType,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub is_active: bool,
    pub version: u64,
}

impl EngineDocument {
    pub fn from_create(input: CreateEngine) -> Result<Self, AppError> {
        let now = DateTime::now();
        Ok(Self {
            id: ObjectId::new(),
            vehicle_model: object_id(&input.vehicle_model.0)?,
            vehicle_generation: object_id(&input.vehicle_generation.0)?,
            vehicle_trims: input
                .vehicle_trims
                .iter()
                .map(|id| object_id(&id.0))
                .collect::<Result<_, _>>()?,
            alloy_material: input.alloy_material,
            factory_code: input.factory_code,
            displacement: input.displacement,
            horsepower: input.horsepower,
            rpm: input.rpm,
            layout: input.layout,
            aspiration_type: input.aspiration_type,
            fuel_delivery: input.fuel_delivery,
            created_at: now,
            updated_at: now,
            is_active: true,
            version: 1,
        })
    }

    pub fn into_domain(self) -> Result<Engine, AppError> {
        Ok(Engine {
            id: EngineId(self.id.to_hex()),
            vehicle_model: VehicleModelId(self.vehicle_model.to_hex()),
            vehicle_generation: GenerationId(self.vehicle_generation.to_hex()),
            vehicle_trims: self
                .vehicle_trims
                .into_iter()
                .map(|id| TrimId(id.to_hex()))
                .collect(),
            alloy_material: self.alloy_material,
            factory_code: self.factory_code,
            displacement: self.displacement,
            horsepower: self.horsepower,
            rpm: self.rpm,
            layout: self.layout,
            aspiration_type: self.aspiration_type,
            fuel_delivery: self.fuel_delivery,
            metadata: metadata(
                self.created_at,
                self.updated_at,
                self.is_active,
                self.version,
            )?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductFitmentDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub product_id: ObjectId,
    pub vehicle_scope: VehicleScope,
    pub system: VehicleSystem,
    pub component: ComponentType,
    pub position: Option<ComponentPosition>,
    pub notes: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub is_active: bool,
    pub version: u64,
}
impl ProductFitmentDocument {
    pub fn from_create(input: CreateProductFitment) -> Result<Self, AppError> {
        let now = DateTime::now();
        Ok(Self {
            id: ObjectId::new(),
            product_id: object_id(&input.product_id.0)?,
            vehicle_scope: input.vehicle_scope,
            system: input.system,
            component: input.component,
            position: input.position,
            notes: input.notes,
            created_at: now,
            updated_at: now,
            is_active: true,
            version: 1,
        })
    }
    pub fn into_domain(self) -> Result<ProductFitment, AppError> {
        Ok(ProductFitment {
            id: ProductFitmentId(self.id.to_hex()),
            product_id: ProductId(self.product_id.to_hex()),
            vehicle_scope: self.vehicle_scope,
            system: self.system,
            component: self.component,
            position: self.position,
            notes: self.notes,
            metadata: metadata(
                self.created_at,
                self.updated_at,
                self.is_active,
                self.version,
            )?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub category_code: String,
    pub parent_id: Option<ObjectId>,
    pub name: String,
    pub description: Option<String>,
    pub attributes: Vec<AttributeDefinition>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub is_active: bool,
    pub version: u64,
}
impl CategoryDocument {
    pub fn from_create(input: CreateCategory) -> Result<Self, AppError> {
        let now = DateTime::now();
        Ok(Self {
            id: ObjectId::new(),
            category_code: input.category_code,
            parent_id: input.parent_id.map(|id| object_id(&id.0)).transpose()?,
            name: input.name,
            description: input.description,
            attributes: input.attributes,
            created_at: now,
            updated_at: now,
            is_active: true,
            version: 1,
        })
    }
    pub fn into_domain(self) -> Result<Category, AppError> {
        Ok(Category {
            id: CategoryId(self.id.to_hex()),
            category_code: self.category_code,
            parent_id: self.parent_id.map(|id| CategoryId(id.to_hex())),
            name: self.name,
            description: self.description,
            attributes: self.attributes,
            metadata: metadata(
                self.created_at,
                self.updated_at,
                self.is_active,
                self.version,
            )?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrimDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub generation_id: ObjectId,
    pub vehicle_models: Vec<ObjectId>,
    pub name: String,
    pub trim_code: String,
    pub timeframe: Timeframe,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub is_active: bool,
    pub version: u64,
}
impl TrimDocument {
    pub fn from_create(input: CreateTrim) -> Result<Self, AppError> {
        let now = DateTime::now();
        Ok(Self {
            id: ObjectId::new(),
            generation_id: object_id(&input.generation_id.0)?,
            vehicle_models: input
                .vehicle_models
                .into_iter()
                .map(|id| object_id(&id.0))
                .collect::<Result<_, _>>()?,
            name: input.name,
            trim_code: input.trim_code,
            timeframe: input.timeframe,
            created_at: now,
            updated_at: now,
            is_active: true,
            version: 1,
        })
    }
    pub fn into_domain(self) -> Result<Trim, AppError> {
        Ok(Trim {
            id: TrimId(self.id.to_hex()),
            generation_id: GenerationId(self.generation_id.to_hex()),
            vehicle_models: self
                .vehicle_models
                .into_iter()
                .map(|id| VehicleModelId(id.to_hex()))
                .collect(),
            name: self.name,
            trim_code: self.trim_code,
            timeframe: self.timeframe,
            metadata: metadata(
                self.created_at,
                self.updated_at,
                self.is_active,
                self.version,
            )?,
        })
    }
}

fn metadata(
    created_at: DateTime,
    updated_at: DateTime,
    is_active: bool,
    version: u64,
) -> Result<EntityMetadata, AppError> {
    let convert = |value: DateTime| {
        chrono::DateTime::from_timestamp_millis(value.timestamp_millis())
            .ok_or_else(|| AppError::InvalidData("timestamp is outside the supported range".into()))
    };
    Ok(EntityMetadata {
        created_at: convert(created_at)?,
        updated_at: convert(updated_at)?,
        is_active,
        version,
    })
}

impl GenerationDocument {
    pub fn from_create(input: CreateGeneration) -> Result<Self, AppError> {
        let now = DateTime::now();
        Ok(Self {
            id: ObjectId::new(),
            vehicle_model_id: object_id(&input.vehicle_model_id.0)?,
            name: input.name,
            generation_code: input.generation_code,
            timeframe: input.timeframe,
            created_at: now,
            updated_at: now,
            is_active: true,
            version: 1,
        })
    }

    pub fn into_domain(self) -> Result<Generation, AppError> {
        let timestamp = |value: DateTime| {
            chrono::DateTime::from_timestamp_millis(value.timestamp_millis()).ok_or_else(|| {
                AppError::InvalidData("timestamp is outside the supported range".into())
            })
        };
        Ok(Generation {
            id: GenerationId(self.id.to_hex()),
            vehicle_model_id: VehicleModelId(self.vehicle_model_id.to_hex()),
            name: self.name,
            generation_code: self.generation_code,
            timeframe: self.timeframe,
            metadata: EntityMetadata {
                created_at: timestamp(self.created_at)?,
                updated_at: timestamp(self.updated_at)?,
                is_active: self.is_active,
                version: self.version,
            },
        })
    }
}

impl VehicleModelDocument {
    pub fn from_create(input: CreateVehicleModel) -> Self {
        let now = DateTime::now();
        Self {
            id: ObjectId::new(),
            name: input.name,
            model_code: input.model_code,
            created_at: now,
            updated_at: now,
            is_active: true,
            version: 1,
        }
    }

    pub fn into_domain(self) -> Result<VehicleModel, AppError> {
        Ok(VehicleModel {
            id: VehicleModelId(self.id.to_hex()),
            name: self.name,
            model_code: self.model_code,
            metadata: EntityMetadata {
                created_at: chrono::DateTime::from_timestamp_millis(
                    self.created_at.timestamp_millis(),
                )
                .ok_or_else(|| {
                    AppError::InvalidData("createdAt is outside the supported range".into())
                })?,
                updated_at: chrono::DateTime::from_timestamp_millis(
                    self.updated_at.timestamp_millis(),
                )
                .ok_or_else(|| {
                    AppError::InvalidData("updatedAt is outside the supported range".into())
                })?,
                is_active: self.is_active,
                version: self.version,
            },
        })
    }
}

impl ProductDocument {
    pub fn from_create(input: CreateProduct) -> Result<Self, AppError> {
        let now = DateTime::now();
        Ok(Self {
            id: ObjectId::new(),
            sku: input.sku,
            model_number: input.model_number,
            brand_id: object_id(&input.brand_id.0)?,
            price: decimal128(&input.price.amount)?,
            currency: input.price.currency,
            name: input.name,
            attributes: input.attributes,
            description: input.description,
            image_name: input.image_name,
            image_urls: input.image_urls,
            sale_type: input.sale_type,
            category_ids: input
                .category_ids
                .into_iter()
                .map(|id| object_id(&id.0))
                .collect::<Result<_, _>>()?,
            created_at: now,
            updated_at: now,
            is_active: true,
            version: 1,
        })
    }

    pub fn into_domain(self) -> Result<Product, AppError> {
        let amount = Decimal::from_str(&self.price.to_string())
            .map_err(|e| AppError::InvalidData(e.to_string()))?;
        Ok(Product {
            id: ProductId(self.id.to_hex()),
            sku: self.sku,
            model_number: self.model_number,
            brand_id: BrandId(self.brand_id.to_hex()),
            price: Money {
                amount,
                currency: self.currency,
            },
            name: self.name,
            attributes: self.attributes,
            description: self.description,
            image_name: self.image_name,
            image_urls: self.image_urls,
            sale_type: self.sale_type,
            category_ids: self
                .category_ids
                .into_iter()
                .map(|id| CategoryId(id.to_hex()))
                .collect(),
            metadata: EntityMetadata {
                created_at: chrono::DateTime::from_timestamp_millis(
                    self.created_at.timestamp_millis(),
                )
                .ok_or_else(|| {
                    AppError::InvalidData("createdAt is outside the supported range".into())
                })?,
                updated_at: chrono::DateTime::from_timestamp_millis(
                    self.updated_at.timestamp_millis(),
                )
                .ok_or_else(|| {
                    AppError::InvalidData("updatedAt is outside the supported range".into())
                })?,
                is_active: self.is_active,
                version: self.version,
            },
        })
    }
}

pub fn decimal128(value: &Decimal) -> Result<Decimal128, AppError> {
    Decimal128::from_str(&value.to_string()).map_err(|e| AppError::InvalidData(e.to_string()))
}

pub fn object_id(value: &str) -> Result<ObjectId, AppError> {
    ObjectId::parse_str(value).map_err(|_| AppError::BadRequest("invalid identifier".into()))
}
