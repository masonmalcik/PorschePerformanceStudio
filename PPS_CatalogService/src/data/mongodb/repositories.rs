use super::documents::{
    decimal128, object_id, BrandDocument, CategoryDocument, EngineDocument, GenerationDocument,
    ProductDocument, ProductFitmentDocument, TrimDocument, VehicleConfigurationDocument,
    VehicleModelDocument,
};
use crate::{
    application::{
        BrandRepository, CategoryRepository, EngineRepository, GenerationRepository,
        ProductFitmentRepository, ProductRepository, TrimRepository,
        VehicleConfigurationRepository, VehicleModelRepository,
    },
    domain::*,
    AppError,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    options::ReturnDocument,
    Database,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct ProductCursor {
    offset: u64,
}

pub struct MongoProductRepository {
    collection: mongodb::Collection<ProductDocument>,
}
impl MongoProductRepository {
    pub fn new(database: Database) -> Self {
        Self {
            collection: database.collection("products"),
        }
    }
}

#[async_trait]
impl ProductRepository for MongoProductRepository {
    async fn list_active(&self) -> Result<Vec<Product>, AppError> {
        self.collection
            .find(doc! {"isActive":true})
            .sort(doc! {"name":1})
            .await?
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(ProductDocument::into_domain)
            .collect()
    }
    async fn search(&self, query: ProductQuery) -> Result<ProductPage, AppError> {
        let mut filter = doc! { "isActive": true };
        if let Some(id) = query.brand_id {
            filter.insert("brandId", object_id(&id.0)?);
        }
        if let Some(id) = query.category_id {
            filter.insert("categoryIds", object_id(&id.0)?);
        }
        if let Some(sale_type) = query.sale_type {
            filter.insert(
                "saleType",
                mongodb::bson::to_bson(&sale_type)
                    .map_err(|e| AppError::InvalidData(e.to_string()))?,
            );
        }
        if query.minimum_price.is_some() || query.maximum_price.is_some() {
            let mut price = Document::new();
            if let Some(value) = query.minimum_price {
                price.insert("$gte", decimal128(&value)?);
            }
            if let Some(value) = query.maximum_price {
                price.insert("$lte", decimal128(&value)?);
            }
            filter.insert("price", price);
        }
        if let Some(search) = query.search {
            let pattern = regex::escape(&search);
            filter.insert(
                "$or",
                vec![
                    doc! { "name": { "$regex": &pattern, "$options": "i" } },
                    doc! { "sku": { "$regex": &pattern, "$options": "i" } },
                    doc! { "modelNumber": { "$regex": &pattern, "$options": "i" } },
                    doc! { "description": { "$regex": &pattern, "$options": "i" } },
                ],
            );
        }
        let offset = query
            .cursor
            .as_deref()
            .map(decode_cursor)
            .transpose()?
            .unwrap_or(0);
        let direction = match query.direction {
            SortDirection::Ascending => 1,
            SortDirection::Descending => -1,
        };
        let sort_field = match query.sort {
            ProductSort::Name => "name",
            ProductSort::Newest => "createdAt",
            ProductSort::Price => "price",
        };
        let mut documents = self
            .collection
            .find(filter)
            .sort(doc! { (sort_field): direction, "_id": direction })
            .skip(offset)
            .limit(i64::from(query.page_size) + 1)
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        let has_more = documents.len() > query.page_size as usize;
        if has_more {
            documents.pop();
        }
        let item_count = documents.len() as u64;
        let items = documents
            .into_iter()
            .map(ProductDocument::into_domain)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = has_more
            .then(|| encode_cursor(offset + item_count))
            .transpose()?;
        Ok(ProductPage { items, next_cursor })
    }

    async fn get(&self, id: ProductId) -> Result<Option<Product>, AppError> {
        self.collection
            .find_one(doc! { "_id": object_id(&id.0)? })
            .await?
            .map(ProductDocument::into_domain)
            .transpose()
    }

    async fn create(&self, input: CreateProduct) -> Result<Product, AppError> {
        let document = ProductDocument::from_create(input)?;
        self.collection
            .insert_one(&document)
            .await
            .map_err(map_duplicate)?;
        document.into_domain()
    }

    async fn update(
        &self,
        id: ProductId,
        input: UpdateProduct,
    ) -> Result<Option<Product>, AppError> {
        let mut set = doc! { "updatedAt": mongodb::bson::DateTime::now() };
        if let Some(value) = input.model_number {
            set.insert("modelNumber", value);
        }
        if let Some(value) = input.price {
            value.validate().map_err(AppError::BadRequest)?;
            set.insert("price", decimal128(&value.amount)?);
            set.insert("currency", value.currency.to_uppercase());
        }
        if let Some(value) = input.name {
            if value.trim().is_empty() {
                return Err(AppError::BadRequest("name is required".into()));
            }
            set.insert("name", value);
        }
        if let Some(value) = input.attributes {
            set.insert(
                "attributes",
                mongodb::bson::to_bson(&value).map_err(|e| AppError::InvalidData(e.to_string()))?,
            );
        }
        if let Some(value) = input.description {
            set.insert("description", value);
        }
        if let Some(value) = input.image_name {
            if value.trim().is_empty() {
                return Err(AppError::BadRequest("imageName is required".into()));
            }
            set.insert("imageName", value);
        }
        if let Some(value) = input.image_urls {
            set.insert("imageUrls", value);
        }
        if let Some(value) = input.sale_type {
            set.insert(
                "saleType",
                mongodb::bson::to_bson(&value).map_err(|e| AppError::InvalidData(e.to_string()))?,
            );
        }
        if let Some(value) = input.category_ids {
            let ids = value
                .into_iter()
                .map(|id| object_id(&id.0))
                .collect::<Result<Vec<_>, _>>()?;
            set.insert("categoryIds", ids);
        }
        if let Some(value) = input.is_active {
            set.insert("isActive", value);
        }
        let update: Document = doc! { "$set": set, "$inc": { "version": 1_i64 } };
        self.collection
            .find_one_and_update(doc! { "_id": object_id(&id.0)? }, update)
            .return_document(ReturnDocument::After)
            .await
            .map_err(map_duplicate)?
            .map(ProductDocument::into_domain)
            .transpose()
    }

    async fn deactivate(&self, id: ProductId) -> Result<bool, AppError> {
        Ok(self.collection.update_one(doc! { "_id": object_id(&id.0)? }, doc! { "$set": { "isActive": false, "updatedAt": mongodb::bson::DateTime::now() }, "$inc": { "version": 1_i64 } }).await?.matched_count == 1)
    }
}

fn encode_cursor(offset: u64) -> Result<String, AppError> {
    let bytes = serde_json::to_vec(&ProductCursor { offset })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn decode_cursor(value: &str) -> Result<u64, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    let cursor: ProductCursor = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    Ok(cursor.offset)
}

pub struct MongoBrandRepository {
    collection: mongodb::Collection<BrandDocument>,
}
impl MongoBrandRepository {
    pub fn new(database: Database) -> Self {
        Self {
            collection: database.collection("brands"),
        }
    }
}

#[async_trait]
impl BrandRepository for MongoBrandRepository {
    async fn is_active(&self, id: &BrandId) -> Result<bool, AppError> {
        Ok(self
            .collection
            .count_documents(doc! { "_id": object_id(&id.0)?, "isActive": true })
            .await?
            == 1)
    }
    async fn list_active(&self) -> Result<Vec<Brand>, AppError> {
        self.collection
            .find(doc! {"isActive":true})
            .sort(doc! {"name":1})
            .await?
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(BrandDocument::into_domain)
            .collect()
    }
    async fn create(&self, input: CreateBrand) -> Result<Brand, AppError> {
        let document = BrandDocument::from_create(input);
        self.collection.insert_one(&document).await.map_err(|e| {
            if e.to_string().contains("E11000") {
                AppError::Conflict("brandCode already exists".into())
            } else {
                AppError::Database(e)
            }
        })?;
        document.into_domain()
    }
}

pub struct MongoCategoryRepository {
    collection: mongodb::Collection<CategoryDocument>,
}
impl MongoCategoryRepository {
    pub fn new(database: Database) -> Self {
        Self {
            collection: database.collection("categories"),
        }
    }
}

#[async_trait]
impl CategoryRepository for MongoCategoryRepository {
    async fn all_active(&self, ids: &[CategoryId]) -> Result<bool, AppError> {
        if ids.is_empty() {
            return Ok(true);
        }
        let unique: std::collections::HashSet<_> = ids.iter().map(|id| id.0.as_str()).collect();
        let object_ids = unique
            .iter()
            .map(|id| object_id(id))
            .collect::<Result<Vec<_>, _>>()?;
        let count = self
            .collection
            .count_documents(doc! { "_id": { "$in": object_ids }, "isActive": true })
            .await?;
        Ok(count == unique.len() as u64)
    }
    async fn create(&self, input: CreateCategory) -> Result<Category, AppError> {
        let document = CategoryDocument::from_create(input)?;
        self.collection
            .insert_one(&document)
            .await
            .map_err(|error| {
                if error.to_string().contains("E11000") {
                    AppError::Conflict("categoryCode already exists".into())
                } else {
                    AppError::Database(error)
                }
            })?;
        document.into_domain()
    }
    async fn list_active(&self) -> Result<Vec<Category>, AppError> {
        self.collection
            .find(doc! {"isActive":true})
            .sort(doc! {"name":1})
            .await?
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(CategoryDocument::into_domain)
            .collect()
    }
}

fn map_duplicate(error: mongodb::error::Error) -> AppError {
    if error.to_string().contains("E11000") {
        AppError::Conflict("sku already exists".into())
    } else {
        AppError::Database(error)
    }
}

pub struct MongoVehicleModelRepository {
    collection: mongodb::Collection<VehicleModelDocument>,
}

impl MongoVehicleModelRepository {
    pub fn new(database: Database) -> Self {
        Self {
            collection: database.collection("vehicle_models"),
        }
    }
}

#[async_trait]
impl VehicleModelRepository for MongoVehicleModelRepository {
    async fn all_active(&self, ids: &[VehicleModelId]) -> Result<bool, AppError> {
        let unique = ids
            .iter()
            .map(|id| id.0.to_ascii_lowercase())
            .collect::<std::collections::HashSet<_>>();
        let object_ids = unique
            .iter()
            .map(|id| object_id(id))
            .collect::<Result<Vec<_>, _>>()?;
        let count = self
            .collection
            .count_documents(doc! { "_id": { "$in": object_ids }, "isActive": true })
            .await?;
        Ok(count == unique.len() as u64)
    }
    async fn create(&self, input: CreateVehicleModel) -> Result<VehicleModel, AppError> {
        let document = VehicleModelDocument::from_create(input);
        self.collection
            .insert_one(&document)
            .await
            .map_err(|error| {
                if error.to_string().contains("E11000") {
                    AppError::Conflict("modelCode already exists".into())
                } else {
                    AppError::Database(error)
                }
            })?;
        document.into_domain()
    }

    async fn list_active(&self) -> Result<Vec<VehicleModel>, AppError> {
        self.collection
            .find(doc! { "isActive": true })
            .sort(doc! { "name": 1, "_id": 1 })
            .await?
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(VehicleModelDocument::into_domain)
            .collect()
    }
}

pub struct MongoGenerationRepository {
    collection: mongodb::Collection<GenerationDocument>,
}
impl MongoGenerationRepository {
    pub fn new(database: Database) -> Self {
        Self {
            collection: database.collection("vehicle_generations"),
        }
    }
}

#[async_trait]
impl GenerationRepository for MongoGenerationRepository {
    async fn get_active(&self, id: &GenerationId) -> Result<Option<Generation>, AppError> {
        self.collection
            .find_one(doc! { "_id": object_id(&id.0)?, "isActive": true })
            .await?
            .map(GenerationDocument::into_domain)
            .transpose()
    }
    async fn create(&self, input: CreateGeneration) -> Result<Generation, AppError> {
        let document = GenerationDocument::from_create(input)?;
        self.collection
            .insert_one(&document)
            .await
            .map_err(|error| {
                if error.to_string().contains("E11000") {
                    AppError::Conflict("generationCode already exists".into())
                } else {
                    AppError::Database(error)
                }
            })?;
        document.into_domain()
    }
    async fn list_active(&self) -> Result<Vec<Generation>, AppError> {
        self.collection
            .find(doc! {"isActive":true})
            .sort(doc! {"name":1})
            .await?
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(GenerationDocument::into_domain)
            .collect()
    }
}

pub struct MongoTrimRepository {
    collection: mongodb::Collection<TrimDocument>,
}
impl MongoTrimRepository {
    pub fn new(database: Database) -> Self {
        Self {
            collection: database.collection("vehicle_trims"),
        }
    }
}
#[async_trait]
impl TrimRepository for MongoTrimRepository {
    async fn create(&self, input: CreateTrim) -> Result<Trim, AppError> {
        let document = TrimDocument::from_create(input)?;
        self.collection
            .insert_one(&document)
            .await
            .map_err(|error| {
                if error.to_string().contains("E11000") {
                    AppError::Conflict("trimCode already exists for this generation".into())
                } else {
                    AppError::Database(error)
                }
            })?;
        document.into_domain()
    }
    async fn list_active(&self) -> Result<Vec<Trim>, AppError> {
        self.collection
            .find(doc! {"isActive":true})
            .sort(doc! {"name":1})
            .await?
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(TrimDocument::into_domain)
            .collect()
    }
    async fn get_active(&self, id: &TrimId) -> Result<Option<Trim>, AppError> {
        match self
            .collection
            .find_one(doc! {"_id": object_id(&id.0)?, "isActive": true})
            .await?
        {
            Some(value) => Ok(Some(value.into_domain()?)),
            None => Ok(None),
        }
    }
}

pub struct MongoEngineRepository {
    collection: mongodb::Collection<EngineDocument>,
}
impl MongoEngineRepository {
    pub fn new(database: Database) -> Self {
        Self {
            collection: database.collection("engines"),
        }
    }
}
#[async_trait]
impl EngineRepository for MongoEngineRepository {
    async fn create(&self, input: CreateEngine) -> Result<crate::domain::Engine, AppError> {
        let document = EngineDocument::from_create(input)?;
        self.collection
            .insert_one(&document)
            .await
            .map_err(|error| {
                if error.to_string().contains("E11000") {
                    AppError::Conflict(
                        "factoryCode already exists for one of the selected trims".into(),
                    )
                } else {
                    AppError::Database(error)
                }
            })?;
        document.into_domain()
    }
}

pub struct MongoVehicleConfigurationRepository {
    collection: mongodb::Collection<VehicleConfigurationDocument>,
}
impl MongoVehicleConfigurationRepository {
    pub fn new(database: Database) -> Self {
        Self {
            collection: database.collection("vehicle_configurations"),
        }
    }
}
#[async_trait]
impl VehicleConfigurationRepository for MongoVehicleConfigurationRepository {
    async fn create(
        &self,
        input: CreateVehicleConfiguration,
    ) -> Result<VehicleConfiguration, AppError> {
        let document = VehicleConfigurationDocument::from_create(input)?;
        self.collection.insert_one(&document).await.map_err(|e| {
            if e.to_string().contains("E11000") {
                AppError::Conflict(
                    "configuration already exists for this trim and model year".into(),
                )
            } else {
                AppError::Database(e)
            }
        })?;
        document.into_domain()
    }
}
pub struct MongoProductFitmentRepository {
    collection: mongodb::Collection<ProductFitmentDocument>,
}
impl MongoProductFitmentRepository {
    pub fn new(database: Database) -> Self {
        Self {
            collection: database.collection("product_fitments"),
        }
    }
}
#[async_trait]
impl ProductFitmentRepository for MongoProductFitmentRepository {
    async fn create(&self, input: CreateProductFitment) -> Result<ProductFitment, AppError> {
        let document = ProductFitmentDocument::from_create(input)?;
        self.collection.insert_one(&document).await.map_err(|e| {
            if e.to_string().contains("E11000") {
                AppError::Conflict("product fitment already exists".into())
            } else {
                AppError::Database(e)
            }
        })?;
        document.into_domain()
    }
}
