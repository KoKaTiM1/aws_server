use crate::models::hardware::HardwarePayload;
use crate::services::mqtt_bus::MqttBusHandle; // Use MqttBusHandle instead of MqttClient
use actix_web::{get, post, web, HttpResponse};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

// Global storage for ESP health status
lazy_static::lazy_static! {
    static ref ESP_HEALTH: Arc<Mutex<HashMap<String, u64>>> = Arc::new(Mutex::new(HashMap::new()));
}

// Existing hardware registration with health tracking
#[post("/hardware")]
pub async fn register_hardware(
    payload: web::Json<HardwarePayload>,
    mqtt_handle: web::Data<MqttBusHandle>, // Change parameter name
) -> HttpResponse {
    tracing::info!(
        "Hardware registration request - ID: {}, Name: {}, Type: {:?}",
        payload.id,
        payload.name,
        payload.sensor_type
    );

    // Update ESP health status
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    {
        let mut health = ESP_HEALTH.lock().unwrap();
        health.insert(payload.id.to_string(), now);
    }

    // Send MQTT ping command instead of publishing JSON
    let cmd_id = format!("reg_{}", uuid::Uuid::new_v4());
    if let Err(e) = mqtt_handle.ping(&payload.id.to_string(), &cmd_id).await {
        tracing::error!("Failed to send MQTT ping: {}", e);
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": "registered",
        "hardware_id": payload.id,
        "name": payload.name
    }))
}

// Heartbeat endpoint - ESP calls this periodically
#[post("/hardware/{esp_id}/heartbeat")]
pub async fn esp_heartbeat(
    path: web::Path<String>,
    mqtt_handle: web::Data<MqttBusHandle>, // Change parameter name
) -> HttpResponse {
    let esp_id = path.into_inner();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    
    tracing::debug!("Heartbeat received from ESP: {}", esp_id);
    
    // Update last seen timestamp
    {
        let mut health = ESP_HEALTH.lock().unwrap();
        health.insert(esp_id.clone(), now);
    }

    // Send ping command to ESP via MQTT
    let cmd_id = format!("hb_{}", uuid::Uuid::new_v4());
    if let Err(e) = mqtt_handle.ping(&esp_id, &cmd_id).await {
        tracing::error!("Failed to send ping command to ESP {}: {}", esp_id, e);
    }

    HttpResponse::Ok().json(json!({
        "status": "heartbeat_received",
        "server_time": now
    }))
}

// Manual recovery command endpoint
#[post("/hardware/{esp_id}/recover")]
pub async fn recover_esp(
    path: web::Path<String>,
    mqtt_handle: web::Data<MqttBusHandle>, // Change parameter name
) -> HttpResponse {
    let esp_id = path.into_inner();
    
    tracing::warn!("Manual recovery triggered for ESP: {}", esp_id);
    
    // Send ping command (recovery attempt)
    let cmd_id = format!("recover_{}", uuid::Uuid::new_v4());
    if let Err(e) = mqtt_handle.ping(&esp_id, &cmd_id).await {
        tracing::error!("Failed to send recovery ping to ESP {}: {}", esp_id, e);
        return HttpResponse::InternalServerError().json(json!({
            "status": "failed",
            "message": "Unable to send recovery commands"
        }));
    }

    HttpResponse::Ok().json(json!({
        "status": "recovery_sent",
        "esp_id": esp_id
    }))
}

// Background health monitor task
pub async fn start_health_monitor(mqtt_handle: MqttBusHandle) { // Change parameter type
    tokio::spawn(async move {
        let mut check_interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            check_interval.tick().await;
            
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let mut failed_esps = Vec::new();
            
            // Check which ESPs haven't sent heartbeat in 60 seconds
            {
                let health = ESP_HEALTH.lock().unwrap();
                for (esp_id, last_seen) in health.iter() {
                    if now - last_seen > 60 {
                        failed_esps.push(esp_id.clone());
                    }
                }
            }
            
            // Try recovery for failed ESPs
            for esp_id in failed_esps {
                tracing::warn!("ESP {} appears offline, attempting recovery", esp_id);
                
                let cmd_id = format!("monitor_{}", uuid::Uuid::new_v4());
                if let Err(e) = mqtt_handle.ping(&esp_id, &cmd_id).await {
                    tracing::error!("Failed to ping ESP {}: {}", esp_id, e);
                }
                
                sleep(Duration::from_secs(10)).await;
            }
        }
    });
}

// Get ESP health status
#[get("/hardware/health")]
pub async fn get_esp_health() -> HttpResponse {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let mut esp_status = Vec::new();
    
    {
        let health = ESP_HEALTH.lock().unwrap();
        for (esp_id, last_seen) in health.iter() {
            let status = if now - last_seen <= 60 { "online" } else { "offline" };
            esp_status.push(json!({
                "esp_id": esp_id,
                "status": status,
                "last_seen": last_seen,
                "seconds_ago": now - last_seen
            }));
        }
    }
    
    HttpResponse::Ok().json(json!({
        "esp_devices": esp_status
    }))
}

// Existing sensor data handler
#[post("/data/sensor")]
pub async fn sensor_data(payload: web::Json<HardwarePayload>) -> HttpResponse {
    tracing::info!(
        "Sensor data received - ID: {}, Type: {:?}, Timestamp: {:?}",
        payload.id,
        payload.sensor_type,
        payload.timestamp
    );

    HttpResponse::Accepted().json(json!({
        "status": "received",
        "timestamp": payload.timestamp
    }))
}