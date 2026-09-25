use super::{BrandApplication, BrandRepository, CategoryApplication, CategoryRepository};
use crate::{
    domain::{Brand, Category, CreateBrand, CreateCategory},
    AppError,
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct CatalogBrandService {
    repository: Arc<dyn BrandRepository>,
}
impl CatalogBrandService {
    pub fn new(repository: Arc<dyn BrandRepository>) -> Self {
        Self { repository }
    }
}
#[async_trait]
impl BrandApplication for CatalogBrandService {
    async fn create(&self, mut input: CreateBrand) -> Result<Brand, AppError> {
        input.brand_code = input.brand_code.trim().to_uppercase();
        input.name = input.name.trim().to_owned();
        input.image_name = input.image_name.trim().to_owned();
        input.validate().map_err(AppError::BadRequest)?;
        self.repository.create(input).await
    }
    async fn list_active(&self) -> Result<Vec<Brand>, AppError> {
        self.repository.list_active().await
    }
}

pub struct CatalogCategoryService {
    repository: Arc<dyn CategoryRepository>,
}
impl CatalogCategoryService {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }
}
#[async_trait]
impl CategoryApplication for CatalogCategoryService {
    async fn create(&self, mut input: CreateCategory) -> Result<Category, AppError> {
        input.category_code = input.category_code.trim().to_uppercase();
        input.name = input.name.trim().to_owned();
        input.validate().map_err(AppError::BadRequest)?;
        self.repository.create(input).await
    }
    async fn list_active(&self) -> Result<Vec<Category>, AppError> {
        self.repository.list_active().await
    }
}
