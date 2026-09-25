use crate::{
    application::{AssetType, ImageStore},
    AppError,
};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

pub struct FileSystemImageStore {
    root: PathBuf,
}

impl FileSystemImageStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

#[async_trait]
impl ImageStore for FileSystemImageStore {
    async fn exists(&self, asset_type: AssetType, image_name: &str) -> Result<bool, AppError> {
        if Path::new(image_name)
            .file_name()
            .and_then(|value| value.to_str())
            != Some(image_name)
        {
            return Ok(false);
        }
        let folder = match asset_type {
            AssetType::Brand => "brands",
            AssetType::PageBackground => "Page-Backgrounds",
            AssetType::Product => "products",
            AssetType::VehicleModel => "vehicle-models",
        };
        Ok(self.root.join(folder).join(image_name).is_file())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn rejects_path_traversal() {
        let store = FileSystemImageStore::new("assets");
        assert!(!store
            .exists(AssetType::Product, "../secret.txt")
            .await
            .unwrap());
    }
}
