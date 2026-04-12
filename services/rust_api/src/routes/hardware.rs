use actix_web::{post, web, HttpResponse, Error};
use crate::models::hardware::HardwarePayload;
use crate::models::dashboard::{DeviceHealth, DeviceActivity, ActivityType};
use crate::routes::dashboard::{register_device, log_device_activity, update_device_status};
use std::time::SystemTime;

#[post("/hardware/register")]
pub async fn register_hardware_device(payload: web::Json<HardwarePayload>) -> Result<HttpResponse, Error> {
    println!("📡 Registering hardware: {:?}", payload);
    
    // Create device health record
    let device_health = DeviceHealth {
        device_id: payload.id,
        device_name: payload.name.clone(),
        status: payload.data.metadata.as_ref()
            .map(|m| m.status.clone())
            .unwrap_or(crate::models::hardware::HardwareStatus::Online),
        last_seen: payload.timestamp,
        uptime_percentage: 95.0, // Default value, calculate based on actual data
        battery_level: Some(85.0), // Extract from sensor data if available
        signal_strength: Some(75.0), // Extract from sensor data if available
        firmware_version: payload.data.metadata.as_ref()
            .map(|m| m.firmware_version.clone())
            .unwrap_or_else(|| "1.0.0".to_string()),
        location: payload.data.metadata.as_ref()
            .and_then(|m| m.location.clone()),
        // New console control fields – default to production & enabled
        mode: Some("production".to_string()),
        heartbeat_enabled: Some(true),
        mqtt_enabled: Some(true),
    };
    
    // Log activity
    let activity = DeviceActivity {
        device_id: payload.id,
        activity_type: ActivityType::StatusUpdate,
        timestamp: SystemTime::now(),
        details: format!("Device {} registered with sensor type {:?}", payload.name, payload.sensor_type),
        data_size: Some(payload.data.values.len() * 8), // Rough estimate
    };
    
    register_device(device_health);
    log_device_activity(activity);
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "registered",
        "device_id": payload.id,
        "name": payload.name,
        "message": "Device successfully registered and added to monitoring"
    })))
}

#[post("/hardware/heartbeat/{device_id}")]
pub async fn hardware_heartbeat(path: web::Path<u32>) -> Result<HttpResponse, Error> {
    let device_id = path.into_inner();
    
    println!("💓 Heartbeat from device {}", device_id);
    
    // Update device status to Online and refresh last_seen
    update_device_status(device_id, crate::models::hardware::HardwareStatus::Online);
    
    // Log heartbeat activity
    let activity = DeviceActivity {
        device_id,
        activity_type: ActivityType::Heartbeat,
        timestamp: SystemTime::now(),
        details: "Device heartbeat received".to_string(),
        data_size: None,
    };
    
    log_device_activity(activity);
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "heartbeat_received",
        "device_id": device_id,
        "timestamp": SystemTime::now()
    })))
}

#[post("/hardware/data")]
pub async fn sensor_data_device(payload: web::Json<HardwarePayload>) -> Result<HttpResponse, Error> {
    println!("📊 Received sensor data from device {}: {:?}", payload.id, payload.data);
    
    // Update device status
    let status = payload.data.metadata.as_ref()
        .map(|m| m.status.clone())
        .unwrap_or(crate::models::hardware::HardwareStatus::Online);
    update_device_status(payload.id, status);
    
    // Log data activity
    let activity = DeviceActivity {
        device_id: payload.id,
        activity_type: ActivityType::DataSent,
        timestamp: payload.timestamp,
        details: format!("Sensor data received: {} values", payload.data.values.len()),
        data_size: Some(payload.data.values.len() * 8),
    };
    
    log_device_activity(activity);
    
    Ok(HttpResponse::Accepted().json(serde_json::json!({
        "status": "received",
        "device_id": payload.id,
        "data_points": payload.data.values.len(),
        "timestamp": payload.timestamp
    })))
}

#[post("/hardware/heartbeat/{device_id}")]
pub async fn device_heartbeat(path: web::Path<u32>) -> Result<HttpResponse, Error> {
    let device_id = path.into_inner();
    println!("💓 Heartbeat from device {}", device_id);
    
    // Update device status
    update_device_status(device_id, crate::models::hardware::HardwareStatus::Online);
    
    // Log heartbeat activity
    let activity = DeviceActivity {
        device_id,
        activity_type: ActivityType::Heartbeat,
        timestamp: SystemTime::now(),
        details: "Device heartbeat received".to_string(),
        data_size: None,
    };
    
    log_device_activity(activity);
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "heartbeat_received",
        "device_id": device_id,
        "timestamp": SystemTime::now()
    })))
}
