use super::{
    AspirationType, EngineId, EngineLayout, EntityMetadata, GenerationId, TrimId, VehicleModelId,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FuelDeliveryType {
    FuelInjected,
    Carbureted,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Engine {
    pub id: EngineId,
    pub vehicle_model: VehicleModelId,
    pub vehicle_generation: GenerationId,
    pub vehicle_trims: Vec<TrimId>,
    pub alloy_material: String,
    pub factory_code: String,
    pub displacement: f64,
    pub horsepower: i32,
    pub rpm: i32,
    pub layout: EngineLayout,
    pub aspiration_type: AspirationType,
    pub fuel_delivery: FuelDeliveryType,
    pub metadata: EntityMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateEngine {
    pub vehicle_model: VehicleModelId,
    pub vehicle_generation: GenerationId,
    pub vehicle_trims: Vec<TrimId>,
    pub alloy_material: String,
    pub factory_code: String,
    pub displacement: f64,
    pub horsepower: i32,
    pub rpm: i32,
    pub layout: EngineLayout,
    pub aspiration_type: AspirationType,
    pub fuel_delivery: FuelDeliveryType,
}

impl CreateEngine {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("vehicleModel", &self.vehicle_model.0),
            ("vehicleGeneration", &self.vehicle_generation.0),
        ] {
            if value.len() != 24 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(format!("{name} must be a 24-character hexadecimal ID"));
            }
        }
        if self.vehicle_trims.is_empty() || self.vehicle_trims.len() > 20 {
            return Err("vehicleTrims must contain between 1 and 20 trim IDs".into());
        }
        let mut unique = std::collections::HashSet::new();
        for id in &self.vehicle_trims {
            if id.0.len() != 24 || !id.0.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err("each vehicleTrims ID must be a 24-character hexadecimal ID".into());
            }
            if !unique.insert(id.0.to_ascii_lowercase()) {
                return Err("vehicleTrims cannot contain duplicate IDs".into());
            }
        }
        if self.alloy_material.trim().is_empty() || self.alloy_material.len() > 100 {
            return Err("alloyMaterial must contain 1 to 100 characters".into());
        }
        if self.factory_code.trim().is_empty() || self.factory_code.len() > 64 {
            return Err("factoryCode must contain 1 to 64 characters".into());
        }
        if !self.displacement.is_finite() || self.displacement <= 0.0 || self.displacement > 20.0 {
            return Err("displacement must be greater than 0 and no more than 20 liters".into());
        }
        if self.horsepower <= 0 || self.rpm <= 0 {
            return Err("horsepower and rpm must be positive integers".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::CreateEngine;
    use serde_json::json;

    #[test]
    fn engine_requires_valid_fields_and_enum_values() {
        let valid = json!({
            "vehicleModel": "68c8b9a713c53c1d8d950e20",
            "vehicleGeneration": "68c8b9a713c53c1d8d950e21",
            "vehicleTrims": ["68c8b9a713c53c1d8d950e22"],
            "alloyMaterial": "Aluminum", "factoryCode": "M97.76",
            "displacement": 3.6, "horsepower": 415, "rpm": 7600, "layout": "F6", "aspirationType": "naturally_aspirated", "fuelDelivery": "fuel_injected"
        });
        assert!(serde_json::from_value::<CreateEngine>(valid.clone())
            .unwrap()
            .validate()
            .is_ok());
        let mut invalid = valid.clone();
        invalid["displacement"] = json!(0);
        assert!(serde_json::from_value::<CreateEngine>(invalid)
            .unwrap()
            .validate()
            .is_err());
        let mut invalid = valid.clone();
        invalid["horsepower"] = json!(0);
        assert!(serde_json::from_value::<CreateEngine>(invalid)
            .unwrap()
            .validate()
            .is_err());
        let mut invalid = valid.clone();
        invalid["rpm"] = json!(-1);
        assert!(serde_json::from_value::<CreateEngine>(invalid)
            .unwrap()
            .validate()
            .is_err());
        let mut invalid = valid;
        invalid["layout"] = json!("V10");
        assert!(serde_json::from_value::<CreateEngine>(invalid).is_err());
    }

    #[test]
    fn engine_accepts_multiple_distinct_trims() {
        let base = json!({
            "vehicleModel": "68c8b9a713c53c1d8d950e20",
            "vehicleGeneration": "68c8b9a713c53c1d8d950e21",
            "vehicleTrims": ["68c8b9a713c53c1d8d950e22", "68c8b9a713c53c1d8d950e23"],
            "alloyMaterial": "Aluminum", "factoryCode": "M97.76",
            "displacement": 3.6, "horsepower": 415, "rpm": 7600, "layout": "F6", "aspirationType": "naturally_aspirated", "fuelDelivery": "carbureted"
        });
        assert!(serde_json::from_value::<CreateEngine>(base.clone())
            .unwrap()
            .validate()
            .is_ok());
        let mut invalid = base.clone();
        invalid["vehicleTrims"] = json!([]);
        assert!(serde_json::from_value::<CreateEngine>(invalid)
            .unwrap()
            .validate()
            .is_err());
        let mut invalid = base;
        invalid["vehicleTrims"] = json!(["68c8b9a713c53c1d8d950e22", "68c8b9a713c53c1d8d950e22"]);
        assert!(serde_json::from_value::<CreateEngine>(invalid)
            .unwrap()
            .validate()
            .is_err());
    }

    #[test]
    fn engine_rejects_unknown_fuel_delivery() {
        let value = json!({
            "vehicleModel": "68c8b9a713c53c1d8d950e20",
            "vehicleGeneration": "68c8b9a713c53c1d8d950e21",
            "vehicleTrims": ["68c8b9a713c53c1d8d950e22"],
            "alloyMaterial": "Aluminum", "factoryCode": "M97.76",
            "displacement": 3.6, "horsepower": 415, "rpm": 7600,
            "layout": "F6", "aspirationType": "naturally_aspirated", "fuelDelivery": "diesel"
        });
        assert!(serde_json::from_value::<CreateEngine>(value).is_err());
    }
}
