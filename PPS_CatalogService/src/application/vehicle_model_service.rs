use super::{VehicleModelApplication, VehicleModelRepository};
use crate::{
    domain::{CreateVehicleModel, VehicleModel, VehicleModelId},
    AppError,
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct CatalogVehicleModelService {
    repository: Arc<dyn VehicleModelRepository>,
}

impl CatalogVehicleModelService {
    pub fn new(repository: Arc<dyn VehicleModelRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl VehicleModelApplication for CatalogVehicleModelService {
    async fn create(&self, mut input: CreateVehicleModel) -> Result<VehicleModel, AppError> {
        input.name = input.name.trim().to_owned();
        input.model_code = input.model_code.trim().to_uppercase();
        input.validate().map_err(AppError::BadRequest)?;
        self.repository.create(input).await
    }

    async fn list_active(&self) -> Result<Vec<VehicleModel>, AppError> {
        self.repository.list_active().await
    }
    async fn get(&self, id: VehicleModelId) -> Result<VehicleModel, AppError> {
        self.repository.get(id).await?.ok_or(AppError::NotFound)
    }
    async fn update(
        &self,
        id: VehicleModelId,
        mut input: CreateVehicleModel,
    ) -> Result<VehicleModel, AppError> {
        input.name = input.name.trim().to_owned();
        input.model_code = input.model_code.trim().to_uppercase();
        input.validate().map_err(AppError::BadRequest)?;
        self.repository
            .update(id, input)
            .await?
            .ok_or(AppError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vehicle_model_requires_a_code() {
        let input = CreateVehicleModel {
            name: "911".into(),
            model_code: " ".into(),
            is_active: None,
        };
        assert!(input.validate().is_err());
    }
}
