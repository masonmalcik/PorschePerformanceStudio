use super::{
    ComponentPosition, ComponentType, EntityMetadata, GenerationId, ProductFitmentId, ProductId,
    Timeframe, TrimId, VehicleSystem,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "scope",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum VehicleScope {
    Generation {
        generation_id: GenerationId,
        timeframe: Option<Timeframe>,
    },
    Trim {
        trim_id: TrimId,
        timeframe: Option<Timeframe>,
    },
    Universal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductFitment {
    pub id: ProductFitmentId,
    pub product_id: ProductId,
    pub vehicle_scope: VehicleScope,
    pub system: VehicleSystem,
    pub component: ComponentType,
    pub position: Option<ComponentPosition>,
    pub notes: Option<String>,
    pub metadata: EntityMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateProductFitment {
    pub product_id: ProductId,
    pub vehicle_scope: VehicleScope,
    pub system: VehicleSystem,
    pub component: ComponentType,
    pub position: Option<ComponentPosition>,
    pub notes: Option<String>,
}
impl CreateProductFitment {
    pub fn validate(&self) -> Result<(), String> {
        let valid = |v: &str| v.len() == 24 && v.bytes().all(|b| b.is_ascii_hexdigit());
        if !valid(&self.product_id.0) {
            return Err("productId must be a 24-character hexadecimal value".into());
        }
        match &self.vehicle_scope {
            VehicleScope::Generation { generation_id, .. } if !valid(&generation_id.0) => {
                return Err("generationId must be a 24-character hexadecimal value".into())
            }
            VehicleScope::Trim { trim_id, .. } if !valid(&trim_id.0) => {
                return Err("trimId must be a 24-character hexadecimal value".into())
            }
            _ => {}
        }
        if self.notes.as_ref().is_some_and(|n| n.len() > 1000) {
            return Err("notes exceeds its maximum length".into());
        }
        Ok(())
    }
}
