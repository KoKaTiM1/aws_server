use actix_web::{post, web, HttpResponse, Error, HttpRequest};
use actix_multipart::Multipart;
use futures_util::TryStreamExt as _;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::fs::OpenOptions;
use tokio::fs;
use sqlx::PgPool;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_sqs::Client as SqsClient;
use crate::models::alert::AlertPayload;
use crate::models::dashboard::{AlertSeverity, DeviceActivity, ActivityType, DeviceHealth};
use crate::models::hardware::HardwareStatus;
use crate::routes::dashboard::{log_alert, log_device_activity, register_device, update_device_status};
use crate::routes::dashboard::{register_device_persistent, log_alert_persistent, log_device_activity_persistent};
use crate::services::{alert_service::AlertService, image_service::ImageService, device_service::DeviceService, sqs_service::SqsService};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn append_detection_csv(
    device_id: u32,
    timestamp: &str,
    sensor_source: Option<&str>,
    message: &str,
    image_path: Option<&str>,
) -> Result<(), std::io::Error> {
    let base_dir = "./serengeti";
    std::fs::create_dir_all(base_dir)?;

    let csv_path = format!("{}/alerts_dataset.csv", base_dir);
    let needs_header = match std::fs::metadata(&csv_path) {
        Ok(meta) => meta.len() == 0,
        Err(_) => true,
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&csv_path)?;

    if needs_header {
        writeln!(file, "device_id,timestamp,sensor_source,message,image_path")?;
    }

    let sensor = sensor_source.unwrap_or("unknown");
    let img = image_path.unwrap_or("");
    writeln!(
        file,
        "{},{},{},{},{}",
        device_id,
        csv_escape(timestamp),
        csv_escape(sensor),
        csv_escape(message),
        csv_escape(img),
    )?;

    Ok(())
}

/// Ensure device is registered before updating status
/// This is needed because devices might send alerts before explicit registration
fn ensure_device_registered(device_id: u32) {
    use crate::routes::dashboard::init_storage;
    init_storage();
    unsafe {
        if let Some(registry) = crate::routes::dashboard::DEVICE_REGISTRY.as_ref() {
            if !registry.contains_key(&device_id) {
                // Device not registered, create a basic entry
                let device = DeviceHealth {
                    device_id,
                    device_name: format!("ESP32-Device-{}", device_id),
                    status: HardwareStatus::Online,
                    last_seen: SystemTime::now(),
                    uptime_percentage: 0.0,
                    battery_level: None,
                    signal_strength: None,
                    firmware_version: "Unknown".to_string(),
                    location: None,
                    mode: Some("production".to_string()),
                    heartbeat_enabled: Some(true),  // Heartbeat required for offline detection
                    mqtt_enabled: Some(true),
                };
                register_device(device);
                println!("✅ Auto-registered device {} from alert", device_id);
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DetectionImage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_base64: Option<String>,           // Optional: base64-encoded image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_raw: Option<Vec<u8>>,             // Optional: raw image bytes
    pub image_format: Option<String>,           // Optional: format for this image (jpg, png, etc.)
}

// Detection alert payload - ONLY for motion/object detection events
#[derive(Debug, Deserialize, Serialize)]
pub struct DetectionAlert {
    // REQUIRED: Core detection data (must be sent immediately)
    pub device_id: u32,                          // Required: Which ESP32 sent this
    pub message: String,                         // Required: Alert description
    pub timestamp: String,                       // Required: When detection occurred
    
    // Optional single / primary Base64 encoded detection image.
    // Old clients can still send it; new clients may omit and use `images` instead.
    pub image_base64: Option<String>,            // Optional: Base64 encoded detection image (backward compatible)
    
    // Optional single / primary raw image bytes.
    // Clients can send raw bytes instead of base64 to skip encoding overhead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_raw: Option<Vec<u8>>,              // Optional: Raw image bytes (backward compatible)
    
    // OPTIONAL: Detection metadata
    pub image_format: Option<String>,            // Optional: jpg, png, etc. (defaults to jpg)
    pub severity: Option<String>,                // Optional: Server can infer from message
    pub sensor_source: Option<String>,           // Optional: PIR / MW / hourly / etc.
    
    // Optional list of additional images; if not sent, defaults to an empty vector.
    #[serde(default)]
    pub images: Vec<DetectionImage>,
}

// Device health/monitoring payload - Sent separately for hardware status updates
#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceHealthUpdate {
    pub device_id: u32,                          // Required: Which ESP32 is reporting
    pub timestamp: String,                       // Required: When status was collected
    pub hardware_data: EspHardwareData,          // Required: All device health metrics
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EspHardwareData {
    // System metrics
    pub cpu_usage_percent: Option<f32>,          // ESP32 CPU utilization
    pub memory_free_kb: Option<u32>,             // Available RAM
    pub temperature_celsius: Option<f32>,        // Internal temperature sensor
    pub uptime_seconds: Option<u64>,             // How long ESP has been running
    
    // Power management
    pub battery_voltage: Option<f32>,            // Battery level in volts
    pub battery_percentage: Option<f32>,         // Calculated battery %
    pub is_charging: Option<bool>,               // USB/external power detected
    pub power_consumption_mw: Option<f32>,       // Current power draw
    
    // Connectivity
    pub wifi_signal_strength: Option<i32>,      // RSSI in dBm
    pub wifi_ssid: Option<String>,               // Connected network
    pub ip_address: Option<String>,              // Current IP
    pub mqtt_connected: Option<bool>,            // MQTT broker status
    
    // Sensors (expandable based on what you add)
    pub light_sensor_lux: Option<f32>,          // Ambient light
    pub humidity_percent: Option<f32>,          // Environmental humidity
    pub pressure_hpa: Option<f32>,              // Barometric pressure
    pub motion_detected: Option<bool>,          // PIR sensor
    pub sound_level_db: Option<f32>,            // Microphone reading
    
    // Location (if GPS enabled)
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude_meters: Option<f32>,
    pub gps_satellites: Option<u8>,
    
    // Device info
    pub firmware_version: Option<String>,
    pub chip_model: Option<String>,             // ESP32, ESP32-S3, etc.
    pub flash_size_mb: Option<u32>,
    pub sketch_size_kb: Option<u32>,            // Current program size
}

#[post("/alerts")]
pub async fn post_alert(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
    s3_client: web::Data<S3Client>,
    s3_bucket: web::Data<String>,
    sqs_client: web::Data<SqsClient>,
    queue_url_ingest: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("multipart/form-data") {
        // Handle multipart form data (with image file)
        return Err(actix_web::error::ErrorBadRequest("Use /alerts/multipart for file uploads"));
    } else {
        // Handle JSON payload (potentially with base64 image)
        handle_json_alert(req, body, pool, s3_client.as_ref(), s3_bucket.as_ref(), sqs_client.as_ref(), queue_url_ingest.as_ref()).await
    }
}

#[post("/alerts/multipart")]
pub async fn post_multipart_alert(
    req: HttpRequest,
    payload: Multipart,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    handle_multipart_alert(req, payload, pool).await
}

async fn handle_json_alert(
    _req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
    s3_client: &S3Client,
    s3_bucket: &str,
    sqs_client: &SqsClient,
    queue_url_ingest: &str,
) -> Result<HttpResponse, Error> {
    println!(" handle_json_alret called, body len={}", body.len(),); 
    // Try to parse as DetectionAlert (motion/object detection with image)
    if let Ok(detection) = serde_json::from_slice::<DetectionAlert>(&body) {
        println!("🎯 [Banana] Detection alert from device {}: {}", 
                 detection.device_id, detection.message);
        
        let device_id = detection.device_id;
        let mut image_path: Option<String> = None;
        
        // Ensure device is registered and mark as online
        ensure_device_registered(device_id);
        update_device_status(device_id, HardwareStatus::Online);
        println!("✅ Device {} marked as online (alert received)", device_id);
        
        // 1) Handle the optional single/primary image (base64 or raw)
        // Try raw image first (more efficient), then fall back to base64
        if let Some(raw_bytes) = detection.image_raw.as_ref() {
            if !raw_bytes.is_empty() {
                // Save the primary/single raw image
                let saved = save_raw_image(
                    raw_bytes.clone(),
                    detection.image_format.clone(),
                    device_id,
                    s3_client,
                    s3_bucket
                ).await?;

                if let Some(ref path) = saved {
                    println!("📸 Saved detection primary raw image (single field): {}", path);
                    image_path = Some(path.clone());
                }
            } else {
                println!("⚠️ Warning: Empty image_raw (single field) received from device {}", device_id);
            }
        } else if let Some(single_b64) = detection.image_base64.as_ref() {
            if !single_b64.is_empty() {
                // Save the primary/single base64 image
                let saved = save_base64_image(
                    single_b64.clone(),
                    detection.image_format.clone(),
                    device_id,
                    s3_client,
                    s3_bucket
                ).await?;

                if let Some(ref path) = saved {
                    println!("📸 Saved detection primary base64 image (single field): {}", path);
                    image_path = Some(path.clone());
                }
            } else {
                println!("⚠️ Warning: Empty image_base64 (single field) received from device {}", device_id);
            }
        } else {
            println!("ℹ️ No single image field provided by device {}; relying on `images` list if present.", device_id);
        }
        
        // 2) Handle the optional list of additional images (both base64 and raw)
        if !detection.images.is_empty() {
            for (idx, img) in detection.images.iter().enumerate() {
                // Per-image format, falling back to top-level image_format if missing
                let per_image_format = img.image_format.clone()
                    .or_else(|| detection.image_format.clone());

                let extra_saved = if let Some(raw_bytes) = img.image_raw.as_ref() {
                    if raw_bytes.is_empty() {
                        println!("⚠️ Skipping empty image_raw in images[{}] from device {}", idx, device_id);
                        continue;
                    }
                    // Save raw image with S3 client
                    save_raw_image(
                        raw_bytes.clone(),
                        per_image_format,
                        device_id,
                        s3_client,
                        s3_bucket
                    ).await?
                } else if let Some(b64_str) = img.image_base64.as_ref() {
                    if b64_str.is_empty() {
                        println!("⚠️ Skipping empty image_base64 in images[{}] from device {}", idx, device_id);
                        continue;
                    }
                    // Save base64 image with S3 client
                    save_base64_image(
                        b64_str.clone(),
                        per_image_format,
                        device_id,
                        s3_client,
                        s3_bucket
                    ).await?
                } else {
                    println!("⚠️ Skipping image[{}] with no data from device {}", idx, device_id);
                    continue;
                };

                if let Some(ref extra_path) = extra_saved {
                    println!("📸 Saved detection extra image [{}]: {}", idx, extra_path);

                    // If no primary path was set yet (no single image, or failed),
                    // use the first successful extra image as the primary one
                    if image_path.is_none() {
                        image_path = Some(extra_path.clone());
                    }
                }
            }
        } else {
            println!("ℹ️ No additional images[] provided by device {}.", device_id);
        }
        
        // Determine severity using service
        let severity = if let Some(severity_str) = &detection.severity {
            match severity_str.as_str() {
                "critical" => AlertSeverity::Critical,
                "high" => AlertSeverity::High,
                "medium" => AlertSeverity::Medium,
                "low" => AlertSeverity::Low,
                _ => AlertService::parse_severity_from_message(&detection.message),
            }
        } else {
            AlertService::parse_severity_from_message(&detection.message)
        };
        
        println!("⚡ ABOUT TO CALL log_alert_to_dashboard for device {}", device_id);
        log_alert_to_dashboard(&**pool, device_id, &detection.message, image_path.clone(), severity).await;
        println!("⚡ RETURNED FROM log_alert_to_dashboard");

        if let Err(e) = append_detection_csv(
            device_id,
            &detection.timestamp,
            detection.sensor_source.as_deref(),
            &detection.message,
            image_path.as_deref(),
        ) {
            println!("⚠️ Failed to append alerts_dataset.csv: {}", e);
        }

        // Publish detection_created event to SQS for processing pipeline
        if !queue_url_ingest.is_empty() {
            let s3_images = if let Some(ref img_path) = image_path {
                vec![img_path.clone()]
            } else {
                vec![]
            };

            if let Err(e) = SqsService::publish_detection_created(
                sqs_client,
                queue_url_ingest,
                device_id,
                s3_images,
                &detection.message,
                &detection.timestamp,
                detection.severity.as_deref().unwrap_or("unknown"),
                detection.sensor_source.as_deref(),
            ).await {
                eprintln!("⚠️ Failed to publish detection_created to SQS: {}", e);
                // Don't fail the request, just log the warning
            }
        }

        return Ok(HttpResponse::Accepted().json(serde_json::json!({
            "status": "detection_received",
            "message": detection.message,
            "device_id": device_id,
            "timestamp": detection.timestamp,
            "image_saved": image_path.is_some(),
            "image_path": image_path,
        })));
    }
    
    // Fallback to legacy AlertPayload format
    let alert: AlertPayload = serde_json::from_slice(&body)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid JSON: {}", e)))?;

    println!("⚠ [Banana] Received legacy alert: {:?}", alert);
    let device_id = 1; // Default device ID for legacy format
    
    // Ensure device is registered and mark as online
    ensure_device_registered(device_id);
    update_device_status(device_id, HardwareStatus::Online);
    
    // Parse severity using service
    let severity = AlertService::parse_severity_from_message(&alert.message);
    
    log_alert_to_dashboard(&**pool, device_id, &alert.message, None, severity).await;

    Ok(HttpResponse::Accepted().json(serde_json::json!({
        "status": "received",
        "message": alert.message,
        "timestamp": alert.timestamp,
    })))
}

async fn save_base64_image(
    base64_data: String,
    image_format: Option<String>,
    device_id: u32,
    s3_client: &S3Client,
    s3_bucket: &str,
) -> Result<Option<String>, Error> {
    let s3_uri = ImageService::save_base64_image(base64_data, image_format, device_id, s3_client, s3_bucket).await?;
    Ok(Some(s3_uri))
}

async fn save_raw_image(
    image_bytes: Vec<u8>,
    image_format: Option<String>,
    device_id: u32,
    s3_client: &S3Client,
    s3_bucket: &str,
) -> Result<Option<String>, Error> {
    let s3_uri = ImageService::save_raw_image(image_bytes, image_format, device_id, s3_client, s3_bucket).await?;
    Ok(Some(s3_uri))
}

async fn handle_multipart_alert(
    _req: HttpRequest,
    mut multipart: Multipart,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let mut alert_data: Option<AlertPayload> = None;
    let mut image_path: Option<String> = None;
    let mut device_id: u32 = 1; // Default device ID
    
    while let Some(mut field) = multipart.try_next().await? {
        let content_disposition = field.content_disposition();
        
        if let Some(name) = content_disposition.and_then(|cd| cd.get_name()) {
            match name {
                "device_id" => {
                    // Extract device_id from form field
                    let mut bytes = web::BytesMut::new();
                    while let Some(chunk) = field.try_next().await? {
                        bytes.extend_from_slice(&chunk);
                    }
                    if let Ok(id_str) = std::str::from_utf8(&bytes) {
                        if let Ok(id) = id_str.trim().parse::<u32>() {
                            device_id = id;
                            println!("📱 Device ID extracted: {}", device_id);
                        }
                    }
                }
                "alert_data" => {
                    // Handle JSON alert data
                    let mut bytes = web::BytesMut::new();
                    while let Some(chunk) = field.try_next().await? {
                        bytes.extend_from_slice(&chunk);
                    }
                    
                    alert_data = Some(serde_json::from_slice::<AlertPayload>(&bytes)
                        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid JSON: {}", e)))?);
                }
                "image" => {
                    // Handle the alert image - save to device-specific folder
                    let timestamp = chrono::Utc::now().timestamp_millis();
                    let file_extension = if let Some(filename) = content_disposition.and_then(|cd| cd.get_filename()) {
                        std::path::Path::new(filename)
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("jpg")
                            .to_string()
                    } else {
                        "jpg".to_string()
                    };
                    
                    // Create device-specific directory
                    let device_photos_dir = format!("./serengeti/esp_photos/{}", device_id);
                    fs::create_dir_all(&device_photos_dir).await.map_err(|e| {
                        actix_web::error::ErrorInternalServerError(format!("Failed to create device photos directory: {}", e))
                    })?;
                    
                    // Set directory permissions to 777 (rwxrwxrwx) - full access
                    if let Ok(dir_metadata) = fs::metadata(&device_photos_dir).await {
                        let mut dir_permissions = dir_metadata.permissions();
                        dir_permissions.set_mode(0o777);
                        let _ = fs::set_permissions(&device_photos_dir, dir_permissions).await;
                    }
                    
                    let file_path = format!("{}/alert_{}.{}", device_photos_dir, timestamp, file_extension);
                    
                    let file_path_clone = file_path.clone();
                    let mut file = web::block(move || std::fs::File::create(&file_path_clone))
                        .await
                        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("File creation error: {}", e)))??;

                    while let Some(chunk) = field.try_next().await? {
                        file = web::block(move || file.write_all(&chunk).map(|_| file))
                            .await
                            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("File write error: {}", e)))??;
                    }
                    
                    // Set file permissions to 666 (rw-rw-rw-) - full read/write access
                    if let Ok(file_metadata) = fs::metadata(&file_path).await {
                        let mut file_permissions = file_metadata.permissions();
                        file_permissions.set_mode(0o666);
                        let _ = fs::set_permissions(&file_path, file_permissions).await;
                    }
                    
                    image_path = Some(file_path.clone());
                    println!("📸 Saved alert image for device {}: {}", device_id, file_path);
                }
                _ => {
                    // Skip unknown fields
                    while let Some(_chunk) = field.try_next().await? {}
                }
            }
        }
    }

    let alert = alert_data.ok_or_else(|| {
        actix_web::error::ErrorBadRequest("Missing alert_data field")
    })?;

    println!("⚠ Received alert with image from device {}: {:?}", device_id, alert);
    
    // Ensure device is registered and mark as online
    ensure_device_registered(device_id);
    update_device_status(device_id, HardwareStatus::Online);
    println!("✅ Device {} marked as online (multipart alert received)", device_id);
    
    // Parse severity using service
    let severity = AlertService::parse_severity_from_message(&alert.message);
    
    log_alert_to_dashboard(&**pool, device_id, &alert.message, image_path.clone(), severity).await;
    
    Ok(HttpResponse::Accepted().json(serde_json::json!({
        "status": "received",
        "message": alert.message,
        "timestamp": alert.timestamp,
        "device_id": device_id,
        "image_path": image_path,
    })))
}

async fn log_alert_to_dashboard(pool: &PgPool, device_id: u32, message: &str, image_path: Option<String>, severity: AlertSeverity) {
    // Create alert summary using service
    let device_name = AlertService::get_device_name_fallback(device_id);
    let alert_summary = AlertService::create_alert_summary(
        device_id,
        &device_name,
        message,
        image_path,
        severity,
    );
    
    let activity = DeviceActivity {
        device_id,
        activity_type: ActivityType::AlertTriggered,
        timestamp: SystemTime::now(),
        details: message.to_string(),
        data_size: None,
    };
    
    // Update in-memory cache
    log_alert(alert_summary.clone());
    log_device_activity(activity.clone());
    
    // Persist to database (await directly - blocking but reliable)
    println!("🔄 [DB] Starting database persistence for device {}", device_id);
    
    // First, ensure device exists in database (required for foreign keys)
    use crate::routes::dashboard::register_device_persistent;
    use crate::models::dashboard::DeviceHealth;
    use crate::models::hardware::HardwareStatus;
    use std::time::SystemTime;
    
    let device_health = DeviceHealth {
        device_id,
        device_name: device_name.clone(),
        status: HardwareStatus::Online,
        last_seen: SystemTime::now(),
        uptime_percentage: 100.0,
        battery_level: None,
        signal_strength: None,
        firmware_version: String::new(),
        location: None,
        mode: Some("detection".to_string()),
        heartbeat_enabled: Some(true),  // Heartbeat required for offline detection
        mqtt_enabled: Some(true),
    };
    register_device_persistent(pool, device_health).await;
    
    // Then persist alert and activity (foreign keys will be satisfied)
    log_alert_persistent(pool, alert_summary).await;
    log_device_activity_persistent(pool, activity).await;
    println!("✅ [DB] Database persistence completed for device {}", device_id);
}

async fn update_device_from_hardware_data(pool: &PgPool, device_id: u32, hardware_data: &EspHardwareData) {
    // Create device health from ESP data using service
    let device_health = DeviceService::create_device_health_from_esp_data(device_id, hardware_data);
    
    // Update in-memory cache
    register_device(device_health.clone());
    
    // Create and log hardware update activity using service
    let activity = DeviceService::create_hardware_update_activity(device_id, hardware_data);
    log_device_activity(activity.clone());
    
    // Async persist to database
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        register_device_persistent(&pool_clone, device_health).await;
    });
    
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        log_device_activity_persistent(&pool_clone, activity).await;
    });
}

// New endpoint for device health/monitoring updates (separate from detections)
#[post("/device/health")]
pub async fn post_device_health(
    body: web::Bytes,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let health_update: DeviceHealthUpdate = serde_json::from_slice(&body)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid JSON: {}", e)))?;

    println!("💓 Device health update from device {}", health_update.device_id);
    
    update_device_from_hardware_data(&**pool, health_update.device_id, &health_update.hardware_data).await;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "health_updated",
        "device_id": health_update.device_id,
        "timestamp": health_update.timestamp,
    })))
}
