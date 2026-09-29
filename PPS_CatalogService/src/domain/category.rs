use super::{CategoryId, EntityMetadata};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeDefinition {
    pub code: String,
    pub name: String,
    pub data_type: AttributeDataType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeDataType {
    Boolean,
    Decimal,
    Integer,
    Text,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: CategoryId,
    pub category_code: String,
    pub parent_id: Option<CategoryId>,
    pub name: String,
    pub description: Option<String>,
    pub attributes: Vec<AttributeDefinition>,
    pub metadata: EntityMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateCategory {
    pub category_code: String,
    pub parent_id: Option<CategoryId>,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub attributes: Vec<AttributeDefinition>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

impl CreateCategory {
    pub fn validate(&self) -> Result<(), String> {
        if self.category_code.trim().is_empty() || self.name.trim().is_empty() {
            return Err("categoryCode and name are required".into());
        }
        if self.category_code.len() > 64 || self.name.len() > 100 {
            return Err("category fields exceed their maximum length".into());
        }
        if let Some(parent) = &self.parent_id {
            if parent.0.len() != 24 || !parent.0.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err("parentId must be a 24-character hexadecimal value".into());
            }
        }
        Ok(())
    }
}
