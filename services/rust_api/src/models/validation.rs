use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct HardwareData {
    #[validate(length(min = 1, max = 100))]
    pub hardware_id: String,
    
    #[validate(range(min = 0.0, max = 100.0))]
    pub cpu_usage: f32,
    
    #[validate(range(min = 0.0, max = 100.0))]
    pub memory_usage: f32,
    
    pub timestamp: DateTime<Utc>,
    
    #[validate(nested)]
    pub sensors: HashMap<String, SensorData>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct SensorData {
    #[validate(range(min = -100.0, max = 100.0))]
    pub temperature: f32,
    
    #[validate(range(min = 0.0, max = 100.0))]
    pub humidity: Option<f32>,
    
    #[validate(range(min = 0, max = 65535))]
    pub light_level: Option<u16>,
    
    pub last_reading: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct AlertData {
    #[validate(length(min = 1, max = 100))]
    pub hardware_id: String,
    
    #[validate(length(min = 1, max = 100))]
    pub alert_type: String,
    
    #[validate(length(min = 1, max = 1000))]
    pub message: String,
    
    pub timestamp: DateTime<Utc>,
    
    #[validate(nested)]
    pub sensor_data: Option<SensorData>,
}

pub fn validate_hardware_data(data: &HardwareData) -> Result<(), String> {
    if let Err(errors) = data.validate() {
        let error_msg = errors.field_errors()
            .iter()
            .map(|(field, errors)| {
                format!("{}: {}", field, errors.iter()
                    .map(|e| e.message.as_ref().unwrap_or(&"Invalid".into()).clone())
                    .collect::<Vec<_>>()
                    .join(", "))
            })
            .collect::<Vec<_>>()
            .join("; ");
            
        tracing::warn!("Hardware data validation failed: {}", error_msg);
        return Err(error_msg);
    }
    
    // Additional custom validations
    if data.timestamp > Utc::now() {
        let msg = "Timestamp cannot be in the future";
        tracing::warn!("Hardware data validation failed: {}", msg);
        return Err(msg.to_string());
    }
    
    for (sensor_id, sensor_data) in &data.sensors {
        if sensor_data.last_reading > Utc::now() {
            let msg = format!("Sensor {} has future timestamp", sensor_id);
            tracing::warn!("Hardware data validation failed: {}", msg);
            return Err(msg);
        }
    }
    
    Ok(())
}

pub fn validate_alert_data(data: &AlertData) -> Result<(), String> {
    if let Err(errors) = data.validate() {
        let error_msg = errors.field_errors()
            .iter()
            .map(|(field, errors)| {
                format!("{}: {}", field, errors.iter()
                    .map(|e| e.message.as_ref().unwrap_or(&"Invalid".into()).clone())
                    .collect::<Vec<_>>()
                    .join(", "))
            })
            .collect::<Vec<_>>()
            .join("; ");
            
        tracing::warn!("Alert data validation failed: {}", error_msg);
        return Err(error_msg);
    }
    
    if data.timestamp > Utc::now() {
        let msg = "Alert timestamp cannot be in the future";
        tracing::warn!("Alert data validation failed: {}", msg);
        return Err(msg.to_string());
    }
    
    Ok(())
}
