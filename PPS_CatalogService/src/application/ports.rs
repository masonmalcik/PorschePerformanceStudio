use crate::{
    domain::{
        Brand, BrandId, Category, CategoryId, CreateBrand, CreateCategory, CreateEngine,
        CreateGeneration, CreateProduct, CreateProductFitment, CreateTrim,
        CreateVehicleConfiguration, CreateVehicleModel, Engine, Generation, Product,
        ProductFitment, ProductId, ProductPage, ProductQuery, Trim, UpdateProduct,
        VehicleConfiguration, VehicleModel,
    },
    AppError,
};
use async_trait::async_trait;

#[async_trait]
pub trait ProductRepository: Send + Sync {
    async fn list_active(&self) -> Result<Vec<Product>, AppError>;
    async fn search(&self, query: ProductQuery) -> Result<ProductPage, AppError>;
    async fn get(&self, id: ProductId) -> Result<Option<Product>, AppError>;
    async fn create(&self, input: CreateProduct) -> Result<Product, AppError>;
    async fn update(
        &self,
        id: ProductId,
        input: UpdateProduct,
    ) -> Result<Option<Product>, AppError>;
    async fn deactivate(&self, id: ProductId) -> Result<bool, AppError>;
}

#[async_trait]
pub trait BrandRepository: Send + Sync {
    async fn is_active(&self, id: &BrandId) -> Result<bool, AppError>;
    async fn list_active(&self) -> Result<Vec<Brand>, AppError>;
    async fn create(&self, input: CreateBrand) -> Result<Brand, AppError>;
}

#[async_trait]
pub trait CategoryRepository: Send + Sync {
    async fn all_active(&self, ids: &[CategoryId]) -> Result<bool, AppError>;
    async fn create(&self, input: CreateCategory) -> Result<Category, AppError>;
    async fn list_active(&self) -> Result<Vec<Category>, AppError>;
}

#[async_trait]
pub trait BrandApplication: Send + Sync {
    async fn list_active(&self) -> Result<Vec<Brand>, AppError>;
    async fn create(&self, input: CreateBrand) -> Result<Brand, AppError>;
}

#[async_trait]
pub trait CategoryApplication: Send + Sync {
    async fn create(&self, input: CreateCategory) -> Result<Category, AppError>;
    async fn list_active(&self) -> Result<Vec<Category>, AppError>;
}

#[derive(Clone, Copy, Debug)]
pub enum AssetType {
    Brand,
    PageBackground,
    Product,
    VehicleModel,
}

#[async_trait]
pub trait ImageStore: Send + Sync {
    async fn exists(&self, asset_type: AssetType, image_name: &str) -> Result<bool, AppError>;
}

#[async_trait]
pub trait ProductApplication: Send + Sync {
    async fn list_active(&self) -> Result<Vec<Product>, AppError>;
    async fn search(&self, query: ProductQuery) -> Result<ProductPage, AppError>;
    async fn get(&self, id: ProductId) -> Result<Product, AppError>;
    async fn create(&self, input: CreateProduct) -> Result<Product, AppError>;
    async fn update(&self, id: ProductId, input: UpdateProduct) -> Result<Product, AppError>;
    async fn deactivate(&self, id: ProductId) -> Result<(), AppError>;
}

#[async_trait]
pub trait VehicleModelRepository: Send + Sync {
    async fn create(&self, input: CreateVehicleModel) -> Result<VehicleModel, AppError>;
    async fn list_active(&self) -> Result<Vec<VehicleModel>, AppError>;
    async fn all_active(&self, ids: &[crate::domain::VehicleModelId]) -> Result<bool, AppError>;
}

#[async_trait]
pub trait VehicleModelApplication: Send + Sync {
    async fn create(&self, input: CreateVehicleModel) -> Result<VehicleModel, AppError>;
    async fn list_active(&self) -> Result<Vec<VehicleModel>, AppError>;
}

#[async_trait]
pub trait GenerationRepository: Send + Sync {
    async fn create(&self, input: CreateGeneration) -> Result<Generation, AppError>;
    async fn list_active(&self) -> Result<Vec<Generation>, AppError>;
    async fn get_active(
        &self,
        id: &crate::domain::GenerationId,
    ) -> Result<Option<Generation>, AppError>;
}

#[async_trait]
pub trait GenerationApplication: Send + Sync {
    async fn create(&self, input: CreateGeneration) -> Result<Generation, AppError>;
    async fn list_active(&self) -> Result<Vec<Generation>, AppError>;
}

#[async_trait]
pub trait TrimRepository: Send + Sync {
    async fn create(&self, input: CreateTrim) -> Result<Trim, AppError>;
    async fn list_active(&self) -> Result<Vec<Trim>, AppError>;
    async fn get_active(&self, id: &crate::domain::TrimId) -> Result<Option<Trim>, AppError>;
}
#[async_trait]
pub trait TrimApplication: Send + Sync {
    async fn create(&self, input: CreateTrim) -> Result<Trim, AppError>;
    async fn list_active(&self) -> Result<Vec<Trim>, AppError>;
}

#[async_trait]
pub trait EngineRepository: Send + Sync {
    async fn create(&self, input: CreateEngine) -> Result<Engine, AppError>;
}

#[async_trait]
pub trait EngineApplication: Send + Sync {
    async fn create(&self, input: CreateEngine) -> Result<Engine, AppError>;
}

#[async_trait]
pub trait VehicleConfigurationRepository: Send + Sync {
    async fn create(
        &self,
        input: CreateVehicleConfiguration,
    ) -> Result<VehicleConfiguration, AppError>;
}
#[async_trait]
pub trait VehicleConfigurationApplication: Send + Sync {
    async fn create(
        &self,
        input: CreateVehicleConfiguration,
    ) -> Result<VehicleConfiguration, AppError>;
}
#[async_trait]
pub trait ProductFitmentRepository: Send + Sync {
    async fn create(&self, input: CreateProductFitment) -> Result<ProductFitment, AppError>;
}
#[async_trait]
pub trait ProductFitmentApplication: Send + Sync {
    async fn create(&self, input: CreateProductFitment) -> Result<ProductFitment, AppError>;
}

#[derive(Clone, Copy, Debug)]
pub enum Permission {
    CatalogAdmin,
}

#[derive(Clone, Debug)]
pub struct RequestIdentity {
    pub development_token: Option<String>,
    pub bearer_token: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Principal {
    pub subject: String,
}

#[async_trait]
pub trait RequestAuthorizer: Send + Sync {
    async fn authorize(
        &self,
        identity: RequestIdentity,
        permission: Permission,
    ) -> Result<Principal, AppError>;
}

#[derive(Clone, Debug)]
pub struct RequestMetric<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub status: u16,
    pub duration_ms: u128,
}

pub trait MetricsSink: Send + Sync {
    fn record_request(&self, metric: RequestMetric<'_>);
    fn increment(&self, name: &'static str);
}
