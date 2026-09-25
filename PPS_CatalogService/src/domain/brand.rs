use super::{BrandId, EntityMetadata};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Brand {
    pub id: BrandId,
    pub brand_code: String,
    pub name: String,
    pub image_name: String,
    pub description: Option<String>,
    pub metadata: EntityMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateBrand {
    pub brand_code: String,
    pub name: String,
    pub image_name: String,
    pub description: Option<String>,
}
impl CreateBrand {
    pub fn validate(&self) -> Result<(), String> {
        if self.brand_code.trim().is_empty()
            || self.name.trim().is_empty()
            || self.image_name.trim().is_empty()
        {
            return Err("brandCode, name, and imageName are required".into());
        }
        if self.brand_code.len() > 32 || self.name.len() > 100 || self.image_name.len() > 255 {
            return Err("brand fields exceed their maximum length".into());
        }
        Ok(())
    }
}
