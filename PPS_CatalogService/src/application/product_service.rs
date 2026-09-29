use super::{
    AssetType, BrandRepository, CategoryRepository, ImageStore, ProductApplication,
    ProductRepository,
};
use crate::{
    domain::{CreateProduct, Product, ProductId, ProductPage, ProductQuery, UpdateProduct},
    AppError,
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct CatalogProductService {
    products: Arc<dyn ProductRepository>,
    brands: Arc<dyn BrandRepository>,
    categories: Arc<dyn CategoryRepository>,
    images: Arc<dyn ImageStore>,
}

impl CatalogProductService {
    pub fn new(
        products: Arc<dyn ProductRepository>,
        brands: Arc<dyn BrandRepository>,
        categories: Arc<dyn CategoryRepository>,
        images: Arc<dyn ImageStore>,
    ) -> Self {
        Self {
            products,
            brands,
            categories,
            images,
        }
    }

    async fn validate_image(&self, image_name: &str) -> Result<(), AppError> {
        if !is_safe_image_name(image_name) {
            return Err(AppError::BadRequest(
                "imageName must be a filesystem-safe filename".into(),
            ));
        }
        if !self.images.exists(AssetType::Product, image_name).await? {
            return Err(AppError::BadRequest(format!(
                "product image does not exist: {image_name}"
            )));
        }
        Ok(())
    }

    async fn validate_categories(&self, ids: &[crate::domain::CategoryId]) -> Result<(), AppError> {
        if !self.categories.all_active(ids).await? {
            return Err(AppError::BadRequest(
                "one or more categories do not exist or are inactive".into(),
            ));
        }
        Ok(())
    }
}

fn is_safe_image_name(value: &str) -> bool {
    !value.trim().is_empty() && !value.contains(['/', '\\']) && value != "." && value != ".."
}

#[async_trait]
impl ProductApplication for CatalogProductService {
    async fn list_active(&self) -> Result<Vec<Product>, AppError> {
        self.products.list_active().await
    }
    async fn search(&self, query: ProductQuery) -> Result<ProductPage, AppError> {
        self.products.search(query).await
    }

    async fn get(&self, id: ProductId) -> Result<Product, AppError> {
        self.products.get(id).await?.ok_or(AppError::NotFound)
    }

    async fn create(&self, mut input: CreateProduct) -> Result<Product, AppError> {
        input.sku = input.sku.trim().to_uppercase();
        input.name = input.name.trim().to_owned();
        input.model_number = input.model_number.map(|value| value.trim().to_owned());
        input.description = input.description.map(|value| value.trim().to_owned());
        input.image_name = input.image_name.trim().to_owned();
        input.price.currency = input.price.currency.to_uppercase();
        input.validate().map_err(AppError::BadRequest)?;
        if !self.brands.is_active(&input.brand_id).await? {
            return Err(AppError::BadRequest(
                "brand does not exist or is inactive".into(),
            ));
        }
        self.validate_categories(&input.category_ids).await?;
        self.validate_image(&input.image_name).await?;
        self.products.create(input).await
    }

    async fn update(&self, id: ProductId, mut input: UpdateProduct) -> Result<Product, AppError> {
        if let Some(sku) = &mut input.sku {
            *sku = sku.trim().to_uppercase();
            if sku.is_empty() {
                return Err(AppError::BadRequest("sku is required".into()));
            }
        }
        if let Some(brand_id) = &input.brand_id {
            if !self.brands.is_active(brand_id).await? {
                return Err(AppError::BadRequest(
                    "brand does not exist or is inactive".into(),
                ));
            }
        }
        if let Some(name) = &mut input.name {
            *name = name.trim().to_owned();
            if name.is_empty() {
                return Err(AppError::BadRequest("name is required".into()));
            }
        }
        if let Some(price) = &mut input.price {
            price.currency = price.currency.to_uppercase();
            price.validate().map_err(AppError::BadRequest)?;
        }
        if let Some(image_name) = &mut input.image_name {
            *image_name = image_name.trim().to_owned();
            self.validate_image(image_name).await?;
        }
        if let Some(ids) = &input.category_ids {
            self.validate_categories(ids).await?;
        }
        self.products
            .update(id, input)
            .await?
            .ok_or(AppError::NotFound)
    }

    async fn deactivate(&self, id: ProductId) -> Result<(), AppError> {
        if self.products.deactivate(id).await? {
            Ok(())
        } else {
            Err(AppError::NotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_safe_image_name;

    #[test]
    fn image_names_cannot_escape_the_asset_folder() {
        assert!(is_safe_image_name("brake-kit.webp"));
        assert!(!is_safe_image_name("../secret.txt"));
        assert!(!is_safe_image_name("nested/image.webp"));
    }
}
