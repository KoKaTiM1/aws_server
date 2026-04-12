use crate::models::dashboard::{AlertSeverity, AlertSummary};
use uuid::Uuid;
use std::time::SystemTime;

pub struct AlertService;

impl AlertService {
    /// Parse alert severity from message content
    pub fn parse_severity_from_message(message: &str) -> AlertSeverity {
        let msg_lower = message.to_lowercase();
        if msg_lower.contains("critical") || msg_lower.contains("emergency") {
            AlertSeverity::Critical
        } else if msg_lower.contains("high") || msg_lower.contains("urgent") {
            AlertSeverity::High
        } else if msg_lower.contains("low") || msg_lower.contains("info") {
            AlertSeverity::Low
        } else {
            AlertSeverity::Medium
        }
    }
    
    /// Create an alert summary from basic components
    pub fn create_alert_summary(
        device_id: u32,
        device_name: &str,
        message: &str,
        image_path: Option<String>,
        severity: AlertSeverity,
    ) -> AlertSummary {
        AlertSummary {
            id: None,
            alert_id: Some(Uuid::new_v4().to_string()),
            device_id,
            device_name: device_name.to_string(),
            message: message.to_string(),
            severity,
            timestamp: SystemTime::now(),
            acknowledged: false,
            image_path,
        }
    }

    /// Get device name from device ID (fallback if device not found)
    pub fn get_device_name_fallback(device_id: u32) -> String {
        format!("ESP32-Device-{}", device_id)
    }

    /// Validate alert message content
    pub fn validate_alert_message(message: &str) -> Result<(), String> {
        if message.trim().is_empty() {
            return Err("Alert message cannot be empty".to_string());
        }
        if message.len() > 1000 {
            return Err("Alert message too long (max 1000 characters)".to_string());
        }
        Ok(())
    }
}