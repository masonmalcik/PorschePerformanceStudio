use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub version: u64,
}

impl EntityMetadata {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            is_active: true,
            version: 1,
        }
    }
}

impl Default for EntityMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeframe {
    pub start_year: u16,
    pub end_year: Option<u16>,
}

impl Timeframe {
    pub fn validate(&self) -> Result<(), String> {
        if !(1900..=2200).contains(&self.start_year) {
            return Err("startYear is outside the supported range".into());
        }
        if self.end_year.is_some_and(|year| year < self.start_year) {
            return Err("endYear cannot precede startYear".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timeframe_rejects_an_end_before_its_start() {
        assert!(Timeframe {
            start_year: 2025,
            end_year: Some(2024)
        }
        .validate()
        .is_err());
    }
}
