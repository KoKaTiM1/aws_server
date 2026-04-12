use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Deserialize, Serialize)]
pub struct HardwarePayload {
    pub id: u32,
    pub name: String,
    pub sensor_type: SensorType,
    #[serde(deserialize_with = "ts_seconds_or_string::deserialize")]
    pub timestamp: SystemTime,
    pub data: SensorData,
}

// Custom serde module for SystemTime supporting ISO 8601 string or seconds
mod ts_seconds_or_string {
    use serde::{self, Deserializer};
    use std::time::{SystemTime, UNIX_EPOCH};
    use chrono::{DateTime, Utc};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = SystemTime;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a unix timestamp (seconds) or ISO 8601 string")
            }
            fn visit_str<E>(self, v: &str) -> Result<SystemTime, E>
            where
                E: serde::de::Error,
            {
                DateTime::parse_from_rfc3339(v)
                    .map(|dt| SystemTime::from(dt.with_timezone(&Utc)))
                    .map_err(|_| E::custom(format!("invalid ISO 8601 timestamp: {v}")))
            }
            fn visit_u64<E>(self, v: u64) -> Result<SystemTime, E>
            where
                E: serde::de::Error,
            {
                Ok(UNIX_EPOCH + std::time::Duration::from_secs(v))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorType {
    #[serde(alias = "Camera", alias = "camera")]
    Camera,
    #[serde(alias = "MotionSensor", alias = "motion_sensor")]
    MotionSensor,
    #[serde(alias = "EnvironmentalSensor", alias = "environmental_sensor")]
    EnvironmentalSensor,
    #[serde(alias = "GPSModule", alias = "gps_module")]
    GPSModule,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SensorData {
    pub values: Vec<f64>,
    pub metadata: Option<SensorMetadata>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SensorMetadata {
    pub location: Option<Location>,
    pub status: HardwareStatus,
    pub firmware_version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HardwareStatus {
    Online,
    Offline,
    Maintenance,
    Error(String),
}
