use super::{GenerationApplication, GenerationRepository};
use crate::{
    domain::{CreateGeneration, Generation, GenerationId},
    AppError,
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct CatalogGenerationService {
    repository: Arc<dyn GenerationRepository>,
}

impl CatalogGenerationService {
    pub fn new(repository: Arc<dyn GenerationRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GenerationApplication for CatalogGenerationService {
    async fn create(&self, mut input: CreateGeneration) -> Result<Generation, AppError> {
        input.name = input.name.trim().to_owned();
        input.generation_code = input.generation_code.trim().to_uppercase();
        input.validate().map_err(AppError::BadRequest)?;
        self.repository.create(input).await
    }
    async fn list_active(&self) -> Result<Vec<Generation>, AppError> {
        self.repository.list_active().await
    }
    async fn get(&self, id: GenerationId) -> Result<Generation, AppError> {
        self.repository.get(id).await?.ok_or(AppError::NotFound)
    }
    async fn update(
        &self,
        id: GenerationId,
        mut input: CreateGeneration,
    ) -> Result<Generation, AppError> {
        input.name = input.name.trim().to_owned();
        input.generation_code = input.generation_code.trim().to_uppercase();
        input.validate().map_err(AppError::BadRequest)?;
        self.repository
            .update(id, input)
            .await?
            .ok_or(AppError::NotFound)
    }
}
