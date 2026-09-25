use super::{
    ProductFitmentApplication, ProductFitmentRepository, VehicleConfigurationApplication,
    VehicleConfigurationRepository,
};
use crate::{
    domain::{
        CreateProductFitment, CreateVehicleConfiguration, ProductFitment, VehicleConfiguration,
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
}
