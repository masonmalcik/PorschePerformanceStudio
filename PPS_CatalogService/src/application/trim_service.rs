use super::{GenerationRepository, TrimApplication, TrimRepository, VehicleModelRepository};
use crate::{
    domain::{CreateTrim, Trim, TrimId},
    AppError,
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct CatalogTrimService {
    repository: Arc<dyn TrimRepository>,
    vehicle_models: Arc<dyn VehicleModelRepository>,
    generations: Arc<dyn GenerationRepository>,
}
impl CatalogTrimService {
    pub fn new(
        repository: Arc<dyn TrimRepository>,
        vehicle_models: Arc<dyn VehicleModelRepository>,
        generations: Arc<dyn GenerationRepository>,
    ) -> Self {
        Self {
            repository,
            vehicle_models,
            generations,
        }
    }
}
#[async_trait]
impl TrimApplication for CatalogTrimService {
    async fn create(&self, mut input: CreateTrim) -> Result<Trim, AppError> {
        input.name = input.name.trim().to_owned();
        input.trim_code = input.trim_code.trim().to_uppercase();
        input.validate().map_err(AppError::BadRequest)?;
        if !self
            .vehicle_models
            .all_active(&input.vehicle_models)
            .await?
        {
            return Err(AppError::BadRequest(
                "one or more vehicleModels do not exist or are inactive".into(),
            ));
        }
        let generation = self
            .generations
            .get_active(&input.generation_id)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest("generationId does not refer to an active generation".into())
            })?;
        if !input
            .vehicle_models
            .iter()
            .any(|id| id.0.eq_ignore_ascii_case(&generation.vehicle_model_id.0))
        {
            return Err(AppError::BadRequest(
                "vehicleModels must include the model associated with generationId".into(),
            ));
        }
        self.repository.create(input).await
    }
    async fn list_active(&self) -> Result<Vec<Trim>, AppError> {
        self.repository.list_active().await
    }
    async fn get(&self, id: TrimId) -> Result<Trim, AppError> {
        self.repository.get(id).await?.ok_or(AppError::NotFound)
    }
    async fn update(&self, id: TrimId, mut input: CreateTrim) -> Result<Trim, AppError> {
        input.name = input.name.trim().to_owned();
        input.trim_code = input.trim_code.trim().to_uppercase();
        input.validate().map_err(AppError::BadRequest)?;
        if !self
            .vehicle_models
            .all_active(&input.vehicle_models)
            .await?
        {
            return Err(AppError::BadRequest(
                "one or more vehicleModels do not exist or are inactive".into(),
            ));
        }
        let generation = self
            .generations
            .get_active(&input.generation_id)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest("generationId does not refer to an active generation".into())
            })?;
        if !input
            .vehicle_models
            .iter()
            .any(|v| v.0.eq_ignore_ascii_case(&generation.vehicle_model_id.0))
        {
            return Err(AppError::BadRequest(
                "vehicleModels must include the model associated with generationId".into(),
            ));
        }
        self.repository
            .update(id, input)
            .await?
            .ok_or(AppError::NotFound)
    }
}
