use super::{
    ProductFitmentApplication, ProductFitmentRepository, VehicleConfigurationApplication,
    VehicleConfigurationRepository,
};
use crate::{
    domain::{
        CreateProductFitment, CreateVehicleConfiguration, ProductFitment, ProductFitmentId,
        VehicleConfiguration, VehicleConfigurationId,
    },
    AppError,
};
use async_trait::async_trait;
use std::sync::Arc;
pub struct CatalogVehicleConfigurationService {
    repository: Arc<dyn VehicleConfigurationRepository>,
}
impl CatalogVehicleConfigurationService {
    pub fn new(repository: Arc<dyn VehicleConfigurationRepository>) -> Self {
        Self { repository }
    }
}
#[async_trait]
impl VehicleConfigurationApplication for CatalogVehicleConfigurationService {
    async fn create(
        &self,
        input: CreateVehicleConfiguration,
    ) -> Result<VehicleConfiguration, AppError> {
        input.validate().map_err(AppError::BadRequest)?;
        self.repository.create(input).await
    }
    async fn list_active(&self) -> Result<Vec<VehicleConfiguration>, AppError> {
        self.repository.list_active().await
    }
    async fn get(&self, id: VehicleConfigurationId) -> Result<VehicleConfiguration, AppError> {
        self.repository.get(id).await?.ok_or(AppError::NotFound)
    }
    async fn update(
        &self,
        id: VehicleConfigurationId,
        input: CreateVehicleConfiguration,
    ) -> Result<VehicleConfiguration, AppError> {
        input.validate().map_err(AppError::BadRequest)?;
        self.repository
            .update(id, input)
            .await?
            .ok_or(AppError::NotFound)
    }
}
pub struct CatalogProductFitmentService {
    repository: Arc<dyn ProductFitmentRepository>,
}
impl CatalogProductFitmentService {
    pub fn new(repository: Arc<dyn ProductFitmentRepository>) -> Self {
        Self { repository }
    }
}
#[async_trait]
impl ProductFitmentApplication for CatalogProductFitmentService {
    async fn create(&self, input: CreateProductFitment) -> Result<ProductFitment, AppError> {
        input.validate().map_err(AppError::BadRequest)?;
        self.repository.create(input).await
    }
    async fn list_active(&self) -> Result<Vec<ProductFitment>, AppError> {
        self.repository.list_active().await
    }
    async fn get(&self, id: ProductFitmentId) -> Result<ProductFitment, AppError> {
        self.repository.get(id).await?.ok_or(AppError::NotFound)
    }
    async fn update(
        &self,
        id: ProductFitmentId,
        input: CreateProductFitment,
    ) -> Result<ProductFitment, AppError> {
        input.validate().map_err(AppError::BadRequest)?;
        self.repository
            .update(id, input)
            .await?
            .ok_or(AppError::NotFound)
    }
}
