/// Database persistence layer for device registry, alerts, and activities
/// 
/// This module provides async functions to persist critical operational data
/// that was previously stored only in-memory.

use sqlx::{PgPool, Row};
use std::time::SystemTime;
use chrono::{DateTime, Utc};
use crate::models::dashboard::{DeviceHealth, AlertSummary, DeviceActivity, AlertSeverity, ActivityType};
use crate::models::hardware::HardwareStatus;

/// Convert SystemTime to DateTime<Utc> for database storage
fn system_time_to_datetime(st: SystemTime) -> DateTime<Utc> {
    let duration = st.duration_since(std::time::UNIX_EPOCH).unwrap();
    DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos()).unwrap()
}

/// Convert DateTime<Utc> to SystemTime for application use
fn datetime_to_system_time(dt: DateTime<Utc>) -> SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64)
}

// ====================================================================
// Device Health Persistence
// ====================================================================

/// Insert or update device health in database
pub async fn upsert_device_health(pool: &PgPool, device: &DeviceHealth) -> Result<(), sqlx::Error> {
    let last_seen = system_time_to_datetime(device.last_seen);
    let location_json = device.location.as_ref()
        .and_then(|loc| serde_json::to_string(loc).ok());
    
    sqlx::query(
        r#"
        INSERT INTO device_health (
            device_id, device_name, status, last_seen, uptime_percentage,
            battery_level, signal_strength, firmware_version, location,
            mode, heartbeat_enabled, mqtt_enabled, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
        ON CONFLICT (device_id) 
        DO UPDATE SET
            device_name = EXCLUDED.device_name,
            status = EXCLUDED.status,
            last_seen = EXCLUDED.last_seen,
            uptime_percentage = EXCLUDED.uptime_percentage,
            battery_level = EXCLUDED.battery_level,
            signal_strength = EXCLUDED.signal_strength,
            firmware_version = EXCLUDED.firmware_version,
            location = EXCLUDED.location,
            mode = EXCLUDED.mode,
            heartbeat_enabled = EXCLUDED.heartbeat_enabled,
            mqtt_enabled = EXCLUDED.mqtt_enabled,
            updated_at = NOW()
        "#
    )
    .bind(device.device_id as i32)
    .bind(&device.device_name)
    .bind(format!("{:?}", device.status))
    .bind(last_seen)
    .bind(device.uptime_percentage)
    .bind(device.battery_level)
    .bind(device.signal_strength)
    .bind(&device.firmware_version)
    .bind(location_json)
    .bind(&device.mode)
    .bind(device.heartbeat_enabled)
    .bind(device.mqtt_enabled)
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Load all devices from database on startup
pub async fn load_all_devices(pool: &PgPool) -> Result<Vec<DeviceHealth>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT 
            device_id, device_name, status, last_seen, uptime_percentage,
            battery_level, signal_strength, firmware_version, location,
            mode, heartbeat_enabled, mqtt_enabled
        FROM device_health
        ORDER BY device_id
        "#
    )
    .fetch_all(pool)
    .await?;
    
    let devices = rows.into_iter().map(|row| {
        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "Online" => HardwareStatus::Online,
            "Offline" => HardwareStatus::Offline,
            "Maintenance" => HardwareStatus::Maintenance,
            "Error" => HardwareStatus::Error("Unknown error".to_string()),
            _ => HardwareStatus::Offline,
        };
        
        let location_json: Option<String> = row.get("location");
        let location = location_json.and_then(|json| serde_json::from_str(&json).ok());
        
        DeviceHealth {
            device_id: row.get::<i32, _>("device_id") as u32,
            device_name: row.get("device_name"),
            status,
            last_seen: datetime_to_system_time(row.get("last_seen")),
            uptime_percentage: row.get("uptime_percentage"),
            battery_level: row.get("battery_level"),
            signal_strength: row.get("signal_strength"),
            firmware_version: row.get("firmware_version"),
            location,
            mode: row.get("mode"),
            heartbeat_enabled: row.get("heartbeat_enabled"),
            mqtt_enabled: row.get("mqtt_enabled"),
        }
    }).collect();
    
    Ok(devices)
}

/// Get single device by ID
pub async fn get_device_by_id(pool: &PgPool, device_id: u32) -> Result<Option<DeviceHealth>, sqlx::Error> {
    let row_opt = sqlx::query(
        r#"
        SELECT 
            device_id, device_name, status, last_seen, uptime_percentage,
            battery_level, signal_strength, firmware_version, location,
            mode, heartbeat_enabled, mqtt_enabled
        FROM device_health
        WHERE device_id = $1
        "#
    )
    .bind(device_id as i32)
    .fetch_optional(pool)
    .await?;
    
    if let Some(row) = row_opt {
        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "Online" => HardwareStatus::Online,
            "Offline" => HardwareStatus::Offline,
            "Maintenance" => HardwareStatus::Maintenance,
            "Error" => HardwareStatus::Error("Unknown error".to_string()),
            _ => HardwareStatus::Offline,
        };
        
        let location_json: Option<String> = row.get("location");
        let location = location_json.and_then(|json| serde_json::from_str(&json).ok());
        
        Ok(Some(DeviceHealth {
            device_id: row.get::<i32, _>("device_id") as u32,
            device_name: row.get("device_name"),
            status,
            last_seen: datetime_to_system_time(row.get("last_seen")),
            uptime_percentage: row.get("uptime_percentage"),
            battery_level: row.get("battery_level"),
            signal_strength: row.get("signal_strength"),
            firmware_version: row.get("firmware_version"),
            location,
            mode: row.get("mode"),
            heartbeat_enabled: row.get("heartbeat_enabled"),
            mqtt_enabled: row.get("mqtt_enabled"),
        }))
    } else {
        Ok(None)
    }
}

// ====================================================================
// Alert History Persistence
// ====================================================================

/// Insert alert into database
pub async fn insert_alert(pool: &PgPool, alert: &AlertSummary) -> Result<(), sqlx::Error> {
    let timestamp = system_time_to_datetime(alert.timestamp);
    
    sqlx::query(
        r#"
        INSERT INTO alert_history (
            device_id, device_name, severity, message, image_path, 
            acknowledged, timestamp
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    )
    .bind(alert.device_id as i32)
    .bind(&alert.device_name)
    .bind(format!("{:?}", alert.severity))
    .bind(&alert.message)
    .bind(&alert.image_path)
    .bind(alert.acknowledged)
    .bind(timestamp)
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Load recent alerts from database (last N alerts)
pub async fn load_recent_alerts(pool: &PgPool, limit: i64) -> Result<Vec<AlertSummary>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT 
            id, device_id, device_name, severity, message, image_path,
            acknowledged, timestamp
        FROM alert_history
        ORDER BY timestamp DESC
        LIMIT $1
        "#
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    let alerts = rows.into_iter().map(|row| {
        let severity_str: String = row.get("severity");
        let severity = match severity_str.as_str() {
            "Critical" => AlertSeverity::Critical,
            "High" => AlertSeverity::High,
            "Medium" => AlertSeverity::Medium,
            "Low" => AlertSeverity::Low,
            _ => AlertSeverity::Medium,
        };
        
        AlertSummary {
            id: Some(row.get::<i32, _>("id") as u32),
            alert_id: Some(row.get::<i32, _>("id").to_string()),
            device_id: row.get::<i32, _>("device_id") as u32,
            device_name: row.get("device_name"),
            severity,
            message: row.get("message"),
            image_path: row.get("image_path"),
            acknowledged: row.get("acknowledged"),
            timestamp: datetime_to_system_time(row.get("timestamp")),
        }
    }).collect();
    
    Ok(alerts)
}

/// Acknowledge an alert by ID
pub async fn acknowledge_alert_db(pool: &PgPool, alert_id: u32) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE alert_history
        SET acknowledged = true, acknowledged_at = NOW()
        WHERE id = $1
        "#
    )
    .bind(alert_id as i32)
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Get alerts for specific device
pub async fn get_alerts_for_device(pool: &PgPool, device_id: u32, limit: i64) -> Result<Vec<AlertSummary>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT 
            id, device_id, device_name, severity, message, image_path,
            acknowledged, timestamp
        FROM alert_history
        WHERE device_id = $1
        ORDER BY timestamp DESC
        LIMIT $2
        "#
    )
    .bind(device_id as i32)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    let alerts = rows.into_iter().map(|row| {
        let severity_str: String = row.get("severity");
        let severity = match severity_str.as_str() {
            "Critical" => AlertSeverity::Critical,
            "High" => AlertSeverity::High,
            "Medium" => AlertSeverity::Medium,
            "Low" => AlertSeverity::Low,
            _ => AlertSeverity::Medium,
        };
        
        AlertSummary {
            id: Some(row.get::<i32, _>("id") as u32),
            alert_id: Some(row.get::<i32, _>("id").to_string()),
            device_id: row.get::<i32, _>("device_id") as u32,
            device_name: row.get("device_name"),
            severity,
            message: row.get("message"),
            image_path: row.get("image_path"),
            acknowledged: row.get("acknowledged"),
            timestamp: datetime_to_system_time(row.get("timestamp")),
        }
    }).collect();
    
    Ok(alerts)
}

// ====================================================================
// Device Activity Persistence
// ====================================================================

/// Insert device activity into database
pub async fn insert_device_activity(pool: &PgPool, activity: &DeviceActivity) -> Result<(), sqlx::Error> {
    let timestamp = system_time_to_datetime(activity.timestamp);
    
    sqlx::query(
        r#"
        INSERT INTO device_activities (
            device_id, activity_type, timestamp, details, data_size
        )
        VALUES ($1, $2, $3, $4, $5)
        "#
    )
    .bind(activity.device_id as i32)
    .bind(format!("{:?}", activity.activity_type))
    .bind(timestamp)
    .bind(&activity.details)
    .bind(activity.data_size.map(|s| s as i64))
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Load recent activities for a device
pub async fn load_device_activities(pool: &PgPool, device_id: u32, limit: i64) -> Result<Vec<DeviceActivity>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT 
            device_id, activity_type, timestamp, details, data_size
        FROM device_activities
        WHERE device_id = $1
        ORDER BY timestamp DESC
        LIMIT $2
        "#
    )
    .bind(device_id as i32)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    let activities = rows.into_iter().map(|row| {
        let activity_type_str: String = row.get("activity_type");
        let activity_type = match activity_type_str.as_str() {
            "DataSent" => ActivityType::DataSent,
            "AlertTriggered" => ActivityType::AlertTriggered,
            "CommandReceived" => ActivityType::CommandReceived,
            "StatusChange" => ActivityType::StatusChange,
            _ => ActivityType::StatusChange,
        };
        
        DeviceActivity {
            device_id: row.get::<i32, _>("device_id") as u32,
            activity_type,
            timestamp: datetime_to_system_time(row.get("timestamp")),
            details: row.get("details"),
            data_size: row.get::<Option<i64>, _>("data_size").map(|s| s as usize),
        }
    }).collect();
    
    Ok(activities)
}

/// Load all recent activities (for dashboard overview)
pub async fn load_all_activities(pool: &PgPool, limit: i64) -> Result<Vec<DeviceActivity>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT 
            device_id, activity_type, timestamp, details, data_size
        FROM device_activities
        ORDER BY timestamp DESC
        LIMIT $1
        "#
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    let activities = rows.into_iter().map(|row| {
        let activity_type_str: String = row.get("activity_type");
        let activity_type = match activity_type_str.as_str() {
            "DataSent" => ActivityType::DataSent,
            "AlertTriggered" => ActivityType::AlertTriggered,
            "CommandReceived" => ActivityType::CommandReceived,
            "StatusChange" => ActivityType::StatusChange,
            _ => ActivityType::StatusChange,
        };
        
        DeviceActivity {
            device_id: row.get::<i32, _>("device_id") as u32,
            activity_type,
            timestamp: datetime_to_system_time(row.get("timestamp")),
            details: row.get("details"),
            data_size: row.get::<Option<i64>, _>("data_size").map(|s| s as usize),
        }
    }).collect();
    
    Ok(activities)
}
