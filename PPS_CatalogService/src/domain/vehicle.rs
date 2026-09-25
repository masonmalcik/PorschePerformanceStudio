use super::{
    EntityMetadata, GenerationId, Timeframe, TrimId, VehicleConfigurationId, VehicleModelId,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleModel {
    pub id: VehicleModelId,
    pub name: String,
    pub model_code: String,
    pub metadata: EntityMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateVehicleModel {
    pub name: String,
    pub model_code: String,
}

impl CreateVehicleModel {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name is required".into());
        }
        if self.model_code.trim().is_empty() {
            return Err("modelCode is required".into());
        }
        if self.name.len() > 100 || self.model_code.len() > 32 {
            return Err("vehicle model fields exceed their maximum length".into());
        }
        if !self
            .model_code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(
                "modelCode may contain only letters, numbers, hyphens, and underscores".into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Generation {
    pub id: GenerationId,
    pub vehicle_model_id: VehicleModelId,
    pub name: String,
    pub generation_code: String,
    pub timeframe: Timeframe,
    pub metadata: EntityMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateGeneration {
    pub vehicle_model_id: VehicleModelId,
    pub name: String,
    pub generation_code: String,
    pub timeframe: Timeframe,
}

impl CreateGeneration {
    pub fn validate(&self) -> Result<(), String> {
        if self.vehicle_model_id.0.len() != 24
            || !self
                .vehicle_model_id
                .0
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("vehicleModelId must be a 24-character hexadecimal value".into());
        }
        if self.name.trim().is_empty() || self.generation_code.trim().is_empty() {
            return Err("name and generationCode are required".into());
        }
        if self.name.len() > 100 || self.generation_code.len() > 32 {
            return Err("generation fields exceed their maximum length".into());
        }
        self.timeframe.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trim {
    pub id: TrimId,
    pub generation_id: GenerationId,
    pub vehicle_models: Vec<VehicleModelId>,
    pub name: String,
    pub trim_code: String,
    pub timeframe: Timeframe,
    pub metadata: EntityMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateTrim {
    pub generation_id: GenerationId,
    pub vehicle_models: Vec<VehicleModelId>,
    pub name: String,
    pub trim_code: String,
    pub timeframe: Timeframe,
}

impl CreateTrim {
    pub fn validate(&self) -> Result<(), String> {
        if self.generation_id.0.len() != 24
            || !self
                .generation_id
                .0
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("generationId must be a 24-character hexadecimal value".into());
        }
        if self.vehicle_models.len() != 1 {
            return Err("vehicleModels must contain exactly one vehicle model ID".into());
        }
        for id in &self.vehicle_models {
            if id.0.len() != 24 || !id.0.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(
                    "each vehicleModels ID must be a 24-character hexadecimal value".into(),
                );
            }
        }
        if self.name.trim().is_empty() || self.trim_code.trim().is_empty() {
            return Err("name and trimCode are required".into());
        }
        if self.name.len() > 100 || self.trim_code.len() > 32 {
            return Err("trim fields exceed their maximum length".into());
        }
        self.timeframe.validate()
    }
}

#[cfg(test)]
mod trim_tests {
    use super::CreateTrim;
    use serde_json::json;

    #[test]
    fn trim_requires_exactly_one_vehicle_model_id() {
        let model_id = "68c8b9a713c53c1d8d950e20";
        let base = json!({ "generationId": "68c8b9a713c53c1d8d950e21", "vehicleModels": [model_id], "name": "GT3", "trimCode": "GT3", "timeframe": { "startYear": 2022, "endYear": null } });
        let valid: CreateTrim = serde_json::from_value(base.clone()).unwrap();
        assert!(valid.validate().is_ok());
        let mut empty = base.clone();
        empty["vehicleModels"] = json!([]);
        assert!(serde_json::from_value::<CreateTrim>(empty)
            .unwrap()
            .validate()
            .is_err());
        let mut multiple = base;
        multiple["vehicleModels"] = json!([model_id, "68c8b9a713c53c1d8d950e22"]);
        assert!(serde_json::from_value::<CreateTrim>(multiple)
            .unwrap()
            .validate()
            .is_err());
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AspirationType {
    NaturallyAspirated,
    Supercharged,
    Turbocharged,
    TwinTurbocharged,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransmissionType {
    Automatic,
    DualClutch,
    Manual,
    Sequential,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DrivetrainType {
    AllWheelDrive,
    FrontWheelDrive,
    RearWheelDrive,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum EngineLayout {
    F6,
    F4,
    I4,
    I5,
    V6,
    V8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineConfiguration {
    pub factory_engine_code: String,
    pub aspiration_type: AspirationType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_layout: Option<EngineLayout>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransmissionConfiguration {
    pub transmission_type: TransmissionType,
    pub factory_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleConfiguration {
    pub id: VehicleConfigurationId,
    pub trim_id: TrimId,
    pub model_year: u16,
    pub engine: Option<EngineConfiguration>,
    pub transmission: Option<TransmissionConfiguration>,
    pub drivetrain: Option<DrivetrainType>,
    pub metadata: EntityMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateVehicleConfiguration {
    pub trim_id: TrimId,
    pub model_year: u16,
    pub engine: Option<EngineConfiguration>,
    pub transmission: Option<TransmissionConfiguration>,
    pub drivetrain: Option<DrivetrainType>,
}
impl CreateVehicleConfiguration {
    pub fn validate(&self) -> Result<(), String> {
        if self.trim_id.0.len() != 24 || !self.trim_id.0.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("trimId must be a 24-character hexadecimal value".into());
        }
        if !(1900..=2200).contains(&self.model_year) {
            return Err("modelYear must be between 1900 and 2200".into());
        }
        if self
            .engine
            .as_ref()
            .is_some_and(|e| e.factory_engine_code.trim().is_empty())
        {
            return Err("factoryEngineCode is required when engine is supplied".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod engine_layout_tests {
    use super::EngineLayout;

    #[test]
    fn engine_layout_accepts_only_supported_codes() {
        for code in ["F6", "F4", "I4", "I5", "V6", "V8"] {
            let layout: EngineLayout = serde_json::from_str(&format!("\"{code}\"")).unwrap();
            assert_eq!(
                serde_json::to_string(&layout).unwrap(),
                format!("\"{code}\"")
            );
        }
        assert!(serde_json::from_str::<EngineLayout>("\"V10\"").is_err());
    }
}
