use super::{
    EngineApplication, EngineRepository, GenerationRepository, TrimRepository,
    VehicleModelRepository,
};
use crate::{
    domain::{CreateEngine, Engine, EngineId},
    AppError,
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct CatalogEngineService {
    repository: Arc<dyn EngineRepository>,
    models: Arc<dyn VehicleModelRepository>,
    generations: Arc<dyn GenerationRepository>,
    trims: Arc<dyn TrimRepository>,
}

impl CatalogEngineService {
    pub fn new(
        repository: Arc<dyn EngineRepository>,
        models: Arc<dyn VehicleModelRepository>,
        generations: Arc<dyn GenerationRepository>,
        trims: Arc<dyn TrimRepository>,
    ) -> Self {
        Self {
            repository,
            models,
            generations,
            trims,
        }
    }
}

#[async_trait]
impl EngineApplication for CatalogEngineService {
    async fn create(&self, mut input: CreateEngine) -> Result<Engine, AppError> {
        input.alloy_material = input.alloy_material.trim().to_owned();
        input.factory_code = input.factory_code.trim().to_uppercase();
        input.validate().map_err(AppError::BadRequest)?;
        if !self
            .models
            .all_active(&[input.vehicle_model.clone()])
            .await?
        {
            return Err(AppError::BadRequest("vehicleModel is not active".into()));
        }
        let generation = self
            .generations
            .get_active(&input.vehicle_generation)
            .await?
            .ok_or_else(|| AppError::BadRequest("vehicleGeneration is not active".into()))?;
        if generation.vehicle_model_id != input.vehicle_model {
            return Err(AppError::BadRequest(
                "vehicleGeneration does not belong to vehicleModel".into(),
            ));
        }
        for trim_id in &input.vehicle_trims {
            let trim = self.trims.get_active(trim_id).await?.ok_or_else(|| {
                AppError::BadRequest(format!("vehicle trim {} is not active", trim_id.0))
            })?;
            if trim.generation_id != input.vehicle_generation
                || !trim.vehicle_models.contains(&input.vehicle_model)
            {
                return Err(AppError::BadRequest(format!(
                    "vehicle trim {} does not belong to the selected model and generation",
                    trim_id.0
                )));
            }
        }
        self.repository.create(input).await
    }
    async fn list_active(&self) -> Result<Vec<Engine>, AppError> {
        self.repository.list_active().await
    }
    async fn get(&self, id: EngineId) -> Result<Engine, AppError> {
        self.repository.get(id).await?.ok_or(AppError::NotFound)
    }
    async fn update(&self, id: EngineId, mut input: CreateEngine) -> Result<Engine, AppError> {
        input.alloy_material = input.alloy_material.trim().to_owned();
        input.factory_code = input.factory_code.trim().to_uppercase();
        input.validate().map_err(AppError::BadRequest)?;
        if !self
            .models
            .all_active(&[input.vehicle_model.clone()])
            .await?
        {
            return Err(AppError::BadRequest("vehicleModel is not active".into()));
        }
        let generation = self
            .generations
            .get_active(&input.vehicle_generation)
            .await?
            .ok_or_else(|| AppError::BadRequest("vehicleGeneration is not active".into()))?;
        if generation.vehicle_model_id != input.vehicle_model {
            return Err(AppError::BadRequest(
                "vehicleGeneration does not belong to vehicleModel".into(),
            ));
        }
        for tid in &input.vehicle_trims {
            let trim = self.trims.get_active(tid).await?.ok_or_else(|| {
                AppError::BadRequest(format!("vehicle trim {} is not active", tid.0))
            })?;
            if trim.generation_id != input.vehicle_generation
                || !trim.vehicle_models.contains(&input.vehicle_model)
            {
                return Err(AppError::BadRequest(format!(
                    "vehicle trim {} does not belong to the selected model and generation",
                    tid.0
                )));
            }
        }
        self.repository
            .update(id, input)
            .await?
            .ok_or(AppError::NotFound)
    }
}
