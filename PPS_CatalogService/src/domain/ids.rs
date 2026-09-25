use serde::{Deserialize, Serialize};

macro_rules! object_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
    };
}

object_id!(BrandId);
object_id!(CategoryId);
object_id!(ProductId);
object_id!(VehicleModelId);
object_id!(GenerationId);
object_id!(TrimId);
object_id!(VehicleConfigurationId);
object_id!(EngineId);
object_id!(ProductFitmentId);
