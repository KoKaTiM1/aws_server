use actix_web::{
    get,
    post,
    web,
    HttpResponse,
    Error,
    Result,
    error::ErrorInternalServerError,
    http::header,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::io::{Read, Write};
use chrono::{DateTime, Utc};
use crate::models::dashboard::{
    DashboardData, DashboardOverview, DeviceHealth, DeviceActivity, 
    DeviceMetrics, AlertSummary, DeviceFilter
};
use crate::models::hardware::{HardwareStatus};
use crate::services::mqtt_bus::MqttBusHandle;
use uuid::Uuid;
use sqlx::PgPool;
use crate::db;
use zip::write::SimpleFileOptions;

#[derive(Debug, Serialize, Deserialize)]
pub struct StartCameraRequest {
    /// Optional resolution string (e.g., "1280x720").
    pub resolution: Option<String>,
    /// Optional frames per second.
    pub fps: Option<u32>,
    /// Optional JPEG quality.
    pub quality: Option<u32>,
    /// Optional maximum stream duration in seconds.
    pub duration_seconds: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CameraResponse {
    /// "success" or "error".
    pub status: String,
    /// Human-readable message.
    pub message: String,
    /// Optional WebSocket URL for the camera stream.
    pub stream_url: Option<String>,
    /// Optional command ID used to correlate with device side.
    pub command_id: Option<String>,
}

/// Payload used by the web console to control a single device's behaviour.
///
/// All fields are optional so the console can toggle only one setting at a time:
/// - `mode`              : "production" or "development"
/// - `heartbeat_enabled` : enabled by default for offline detection (2-hour threshold)
/// - `mqtt_enabled`      : when false, server should ignore MQTT telemetry
#[derive(Debug, Deserialize)]
pub struct DeviceControlPayload {
    /// Optional logical mode of the device: "production" or "development".
    pub mode: Option<String>,

    /// Optional flag indicating whether the server should accept heartbeats.
    /// When `Some(false)` the server may ignore heartbeat messages.
    pub heartbeat_enabled: Option<bool>,

    /// Optional flag indicating whether the server should accept MQTT telemetry.
    /// When `Some(false)` the server may ignore MQTT messages.
    pub mqtt_enabled: Option<bool>,
}

// In-memory storage for demo purposes - replace with database in production
pub static mut DEVICE_REGISTRY: Option<HashMap<u32, DeviceHealth>> = None;
static mut DEVICE_ACTIVITIES: Option<Vec<DeviceActivity>> = None;
static mut ALERT_HISTORY: Option<Vec<AlertSummary>> = None;

/// Ensure the global in-memory registries are initialised.
pub fn init_storage() {
    unsafe {
        if DEVICE_REGISTRY.is_none() {
            DEVICE_REGISTRY = Some(HashMap::new());
            DEVICE_ACTIVITIES = Some(Vec::new());
            ALERT_HISTORY = Some(Vec::new());
        }
    }
}

/// Load persistent data from database into memory cache on startup
pub async fn load_from_database(pool: &PgPool) {
    init_storage();
    
    // Load devices
    match db::load_all_devices(pool).await {
        Ok(devices) => {
            unsafe {
                let registry = DEVICE_REGISTRY.as_mut().unwrap();
                for device in devices {
                    registry.insert(device.device_id, device);
                }
            }
            println!("✅ Loaded {} devices from database", unsafe { DEVICE_REGISTRY.as_ref().unwrap().len() });
        }
        Err(e) => eprintln!("❌ Failed to load devices: {}", e),
    }
    
    // Load recent alerts
    match db::load_recent_alerts(pool, 500).await {
        Ok(alerts) => {
            unsafe {
                *ALERT_HISTORY.as_mut().unwrap() = alerts;
            }
            println!("✅ Loaded {} alerts from database", unsafe { ALERT_HISTORY.as_ref().unwrap().len() });
        }
        Err(e) => eprintln!("❌ Failed to load alerts: {}", e),
    }
    
    // Load recent activities
    match db::load_all_activities(pool, 1000).await {
        Ok(activities) => {
            unsafe {
                *DEVICE_ACTIVITIES.as_mut().unwrap() = activities;
            }
            println!("✅ Loaded {} activities from database", unsafe { DEVICE_ACTIVITIES.as_ref().unwrap().len() });
        }
        Err(e) => eprintln!("❌ Failed to load activities: {}", e),
    }

    // Fallback so devices can still appear after restart when DB has not yet been populated.
    let recovered = recover_devices_from_detection_csv();
    if recovered > 0 {
        println!("✅ Recovered {} devices from detection CSV", recovered);
    }
}

fn parse_csv_timestamp(ts: &str) -> SystemTime {
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        let utc = dt.with_timezone(&Utc);
        return UNIX_EPOCH
            + std::time::Duration::from_secs(utc.timestamp() as u64)
            + std::time::Duration::from_nanos(utc.timestamp_subsec_nanos() as u64);
    }

    if let Ok(dt) = DateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S%.f %z") {
        let utc = dt.with_timezone(&Utc);
        return UNIX_EPOCH
            + std::time::Duration::from_secs(utc.timestamp() as u64)
            + std::time::Duration::from_nanos(utc.timestamp_subsec_nanos() as u64);
    }

    SystemTime::now()
}

fn infer_sensor_source(sensor_source: Option<&str>, message: &str) -> String {
    if let Some(src) = sensor_source {
        let normalized = src.trim().to_lowercase();
        if !normalized.is_empty() && normalized != "unknown" && normalized != "n/a" {
            if normalized.contains("pir") {
                return "pir".to_string();
            }
            if normalized.contains("microwave") || normalized == "mw" {
                return "mw".to_string();
            }
            if normalized.contains("hourly") || normalized.contains("alive") {
                return "hourly_alive".to_string();
            }
            return src.trim().to_string();
        }
    }

    let msg = message.to_lowercase();
    if msg.contains("pir") {
        "pir".to_string()
    } else if msg.contains("microwave") || msg.contains("mw") {
        "mw".to_string()
    } else if msg.contains("hourly") || msg.contains("alive") {
        "hourly_alive".to_string()
    } else {
        "unknown".to_string()
    }
}

fn csv_cell(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn photo_path_to_url(image_path: &str) -> Option<String> {
    if image_path.trim().is_empty() {
        return None;
    }

    let normalized = image_path.replace('\\', "/");
    if normalized.starts_with("/api/v1/photos/") {
        return Some(normalized);
    }

    if let Some(pos) = normalized.find("serengeti/esp_photos/") {
        let suffix = &normalized[(pos + "serengeti/esp_photos/".len())..];
        let suffix = suffix.trim_start_matches('/');
        if !suffix.is_empty() {
            return Some(format!("/api/v1/photos/{}", suffix));
        }
    }

    None
}

fn recover_devices_from_detection_csv() -> usize {
    let csv_path = "./serengeti/alerts_dataset.csv";
    let mut rdr = match csv::ReaderBuilder::new().has_headers(true).from_path(csv_path) {
        Ok(reader) => reader,
        Err(_) => return 0,
    };

    let headers = match rdr.headers() {
        Ok(h) => h.clone(),
        Err(_) => return 0,
    };

    let idx_device_id = headers.iter().position(|h| h == "device_id").unwrap_or(0);
    let idx_timestamp = headers.iter().position(|h| h == "timestamp").unwrap_or(1);

    let mut last_seen_by_device: HashMap<u32, SystemTime> = HashMap::new();

    for rec in rdr.records().flatten() {
        let Some(device_raw) = rec.get(idx_device_id) else { continue; };
        let Ok(device_id) = device_raw.parse::<u32>() else { continue; };
        let parsed_ts = parse_csv_timestamp(rec.get(idx_timestamp).unwrap_or(""));

        let current = last_seen_by_device.entry(device_id).or_insert(parsed_ts);
        if parsed_ts > *current {
            *current = parsed_ts;
        }
    }

    if last_seen_by_device.is_empty() {
        return 0;
    }

    let mut recovered_count = 0usize;
    unsafe {
        if let Some(registry) = DEVICE_REGISTRY.as_mut() {
            for (device_id, last_seen) in last_seen_by_device {
                if let Some(existing) = registry.get_mut(&device_id) {
                    if last_seen > existing.last_seen {
                        existing.last_seen = last_seen;
                    }
                    continue;
                }

                registry.insert(
                    device_id,
                    DeviceHealth {
                        device_id,
                        device_name: format!("ESP32-Device-{}", device_id),
                        status: HardwareStatus::Offline,
                        last_seen,
                        uptime_percentage: 0.0,
                        battery_level: None,
                        signal_strength: None,
                        firmware_version: "Unknown".to_string(),
                        location: None,
                        mode: Some("production".to_string()),
                        heartbeat_enabled: Some(true),
                        mqtt_enabled: Some(true),
                    },
                );
                recovered_count += 1;
            }
        }
    }

    recovered_count
}

/// Current system time helper.
fn get_current_time() -> SystemTime {
    SystemTime::now()
}

#[get("/dashboard")]
pub async fn get_dashboard() -> Result<HttpResponse, Error> {
    init_storage();
    
    unsafe {
        let devices = DEVICE_REGISTRY.as_ref().unwrap();
        let alerts = ALERT_HISTORY.as_ref().unwrap();
        
        let total_devices = devices.len() as u32;
        let online_devices = devices.values()
            .filter(|d| matches!(d.status, HardwareStatus::Online))
            .count() as u32;
        let offline_devices = total_devices - online_devices;
        
        let now = get_current_time();
        let today_start = UNIX_EPOCH + std::time::Duration::from_secs(
            (now.duration_since(UNIX_EPOCH).unwrap().as_secs() / 86400) * 86400
        );
        
        let total_alerts_today = alerts.iter()
            .filter(|a| a.timestamp >= today_start)
            .count() as u32;
            
        let devices_with_alerts = alerts.iter()
            .filter(|a| a.timestamp >= today_start)
            .map(|a| a.device_id)
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;
        
        let average_uptime = if total_devices > 0 {
            devices.values().map(|d| d.uptime_percentage).sum::<f64>() / total_devices as f64
        } else {
            0.0
        };
        
        let overview = DashboardOverview {
            total_devices,
            online_devices,
            offline_devices,
            devices_with_alerts,
            total_alerts_today,
            // Placeholder for now – can be wired to real stats later.
            total_data_received_mb: 42.5,
            average_uptime,
            last_updated: now,
        };
        
        let device_health: Vec<DeviceHealth> = devices.values().cloned().collect();
        
        let mut recent_alerts = alerts.clone();
        recent_alerts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        recent_alerts.truncate(20);
        
        let device_metrics = get_device_metrics().await;
        
        let dashboard_data = DashboardData {
            overview,
            device_health,
            recent_alerts,
            device_metrics,
        };
        
        Ok(HttpResponse::Ok().json(dashboard_data))
    }
}

#[get("/dashboard/overview")]
pub async fn get_dashboard_overview() -> Result<HttpResponse, Error> {
    init_storage();
    
    unsafe {
        let devices = DEVICE_REGISTRY.as_ref().unwrap();
        let alerts = ALERT_HISTORY.as_ref().unwrap();
        
        let total_devices = devices.len() as u32;
        let online_devices = devices.values()
            .filter(|d| matches!(d.status, HardwareStatus::Online))
            .count() as u32;
        let offline_devices = total_devices - online_devices;
        
        let now = get_current_time();
        let today_start = UNIX_EPOCH + std::time::Duration::from_secs(
            (now.duration_since(UNIX_EPOCH).unwrap().as_secs() / 86400) * 86400
        );
        
        let total_alerts_today = alerts.iter()
            .filter(|a| a.timestamp >= today_start)
            .count() as u32;
            
        let devices_with_alerts = alerts.iter()
            .filter(|a| a.timestamp >= today_start)
            .map(|a| a.device_id)
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;
        
        let average_uptime = if total_devices > 0 {
            devices.values().map(|d| d.uptime_percentage).sum::<f64>() / total_devices as f64
        } else {
            0.0
        };
        
        let overview = DashboardOverview {
            total_devices,
            online_devices,
            offline_devices,
            devices_with_alerts,
            total_alerts_today,
            // Mock data for now – can be wired to real aggregation.
            total_data_received_mb: 245.7,
            average_uptime,
            last_updated: now,
        };
        
        Ok(HttpResponse::Ok().json(overview))
    }
}

#[get("/dashboard/devices")]
pub async fn get_all_device_health() -> Result<HttpResponse, Error> {
    init_storage();
    
    unsafe {
        let devices: Vec<DeviceHealth> =
            DEVICE_REGISTRY.as_ref().unwrap().values().cloned().collect();
        Ok(HttpResponse::Ok().json(devices))
    }
}

#[get("/dashboard/devices/{device_id}")]
pub async fn get_device_health(path: web::Path<u32>) -> Result<HttpResponse, Error> {
    init_storage();
    let device_id = path.into_inner();
    
    unsafe {
        if let Some(device) = DEVICE_REGISTRY.as_ref().unwrap().get(&device_id) {
            Ok(HttpResponse::Ok().json(device))
        } else {
            Ok(HttpResponse::NotFound().json(json!({
                "error": "Device not found",
                "device_id": device_id
            })))
        }
    }
}

/// POST /api/v1/devices/{device_id}/control
///
/// This endpoint is called by the console UI when toggling:
///  - Development / Production mode
///  - "Accept heartbeats"
///  - "Accept MQTT telemetry"
///
/// It updates the in-memory DEVICE_REGISTRY so the rest of the system
/// can respect these settings (for example, heartbeat and MQTT services
/// can choose to ignore messages from development devices).
#[post("/devices/{device_id}/control")]
pub async fn control_device(
    path: web::Path<u32>,
    payload: web::Json<DeviceControlPayload>,
) -> Result<HttpResponse, Error> {
    let device_id = path.into_inner();
    let payload = payload.into_inner();

    init_storage();

    unsafe {
        let registry = DEVICE_REGISTRY.as_mut().unwrap();

        // Ensure the device exists
        let device = match registry.get_mut(&device_id) {
            Some(device) => device,
            None => {
                return Ok(HttpResponse::NotFound().json(json!({
                    "error": "Device not found",
                    "device_id": device_id
                })));
            }
        };

        // Apply mode change if requested
        if let Some(mode) = payload.mode {
            device.mode = Some(mode);
        }

        // Apply heartbeat flag if requested
        if let Some(enabled) = payload.heartbeat_enabled {
            device.heartbeat_enabled = Some(enabled);
        }

        // Apply MQTT flag if requested
        if let Some(enabled) = payload.mqtt_enabled {
            device.mqtt_enabled = Some(enabled);
        }
    }

    Ok(HttpResponse::Ok().json(json!({
        "status": "ok",
        "device_id": device_id
    })))
}

#[get("/dashboard/devices/{device_id}/metrics")]
pub async fn get_device_metrics_by_id(path: web::Path<u32>) -> Result<HttpResponse, Error> {
    init_storage();
    let device_id = path.into_inner();
    
    unsafe {
        let activities = DEVICE_ACTIVITIES.as_ref().unwrap();
        let device_activities: Vec<DeviceActivity> = activities.iter()
            .filter(|a| a.device_id == device_id)
            .take(24)
            .cloned()
            .collect();
        
        let alerts_count = ALERT_HISTORY.as_ref().unwrap().iter()
            .filter(|a| a.device_id == device_id)
            .count() as u32;
        
        let metrics = DeviceMetrics {
            device_id,
            cpu_usage: Some(45.2),
            memory_usage: Some(67.8),
            temperature: Some(42.1),
            data_sent_mb: 156.3,
            alerts_count,
            last_24h_activity: device_activities,
        };
        
        Ok(HttpResponse::Ok().json(metrics))
    }
}

#[get("/dashboard/devices/{device_id}/activity")]
pub async fn get_device_activity(path: web::Path<u32>) -> Result<HttpResponse, Error> {
    init_storage();
    let device_id = path.into_inner();
    
    unsafe {
        let activities: Vec<DeviceActivity> = DEVICE_ACTIVITIES
            .as_ref()
            .unwrap()
            .iter()
            .filter(|a| a.device_id == device_id)
            .cloned()
            .collect();
        
        Ok(HttpResponse::Ok().json(activities))
    }
}

#[derive(Debug, Serialize)]
pub struct DevicePhoto {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified: SystemTime,
    pub url: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DeviceDetectionRow {
    pub device_id: u32,
    pub timestamp: String,
    pub sensor_source: String,
    pub message: String,
    pub image_path: Option<String>,
    pub image_url: Option<String>,
}

fn collect_device_detections(device_id: u32, limit: usize) -> Vec<DeviceDetectionRow> {
    let csv_path = "./serengeti/alerts_dataset.csv";

    let mut rdr = match csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(csv_path)
    {
        Ok(reader) => reader,
        Err(_) => return Vec::new(),
    };

    let headers = match rdr.headers() {
        Ok(h) => h.clone(),
        Err(_) => return Vec::new(),
    };

    let idx_device_id = headers.iter().position(|h| h == "device_id").unwrap_or(0);
    let idx_timestamp = headers.iter().position(|h| h == "timestamp").unwrap_or(1);
    let idx_sensor = headers.iter().position(|h| h == "sensor_source");
    let idx_message = headers
        .iter()
        .position(|h| h == "message")
        .unwrap_or_else(|| if idx_sensor.is_some() { 3 } else { 2 });
    let idx_image = headers
        .iter()
        .position(|h| h == "image_path")
        .unwrap_or_else(|| if idx_sensor.is_some() { 4 } else { 3 });

    let header_has_id_prefix = headers
        .get(0)
        .map(|h| h.eq_ignore_ascii_case("id"))
        .unwrap_or(false);

    let pick_first = |rec: &csv::StringRecord, candidates: &[usize]| -> Option<String> {
        for idx in candidates {
            if let Some(v) = rec.get(*idx) {
                if !v.trim().is_empty() {
                    return Some(v.to_string());
                }
            }
        }
        None
    };

    let mut rows: Vec<(SystemTime, DeviceDetectionRow)> = Vec::new();

    for rec in rdr.records().flatten() {
        let compact_layout = rec.len() <= 5;

        let device_candidates: Vec<usize> = if compact_layout {
            vec![idx_device_id, 0]
        } else if header_has_id_prefix {
            vec![idx_device_id, 2]
        } else {
            vec![idx_device_id, 0]
        };

        let mut parsed_device_id: Option<u32> = None;
        for idx in &device_candidates {
            if let Some(raw) = rec.get(*idx) {
                if let Ok(v) = raw.trim().parse::<u32>() {
                    parsed_device_id = Some(v);
                    break;
                }
            }
        }
        let Some(row_device_id) = parsed_device_id else { continue; };
        if row_device_id != device_id {
            continue;
        }

        let timestamp = pick_first(&rec, &[idx_timestamp, 1]).unwrap_or_default();
        let message_candidates: Vec<usize> = if compact_layout {
            vec![idx_message, 3]
        } else if header_has_id_prefix {
            vec![idx_message, 7, 3]
        } else {
            vec![idx_message, 3]
        };
        let message = pick_first(&rec, &message_candidates).unwrap_or_default();
        let sensor_source = infer_sensor_source(
            idx_sensor
                .and_then(|i| rec.get(i))
                .or_else(|| {
                    if compact_layout {
                        rec.get(2)
                    } else if header_has_id_prefix {
                        rec.get(6).or_else(|| rec.get(2))
                    } else {
                        rec.get(2)
                    }
                }),
            &message,
        );
        let image_candidates: Vec<usize> = if compact_layout {
            vec![idx_image, 4]
        } else if header_has_id_prefix {
            vec![idx_image, 8, 4]
        } else {
            vec![idx_image, 4]
        };
        let image_path = pick_first(&rec, &image_candidates)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let image_url = image_path.as_ref().and_then(|p| photo_path_to_url(p));

        rows.push((
            parse_csv_timestamp(&timestamp),
            DeviceDetectionRow {
                device_id,
                timestamp,
                sensor_source,
                message,
                image_path,
                image_url,
            },
        ));
    }

    rows.sort_by(|a, b| b.0.cmp(&a.0));
    rows.into_iter().take(limit).map(|(_, row)| row).collect()
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::io::Cursor<Vec<u8>>>,
    root: &std::path::Path,
    current: &std::path::Path,
) -> std::io::Result<usize> {
    let mut count = 0usize;
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            count += add_dir_to_zip(zip, root, &path)?;
            continue;
        }

        if !metadata.is_file() {
            continue;
        }

        let rel = match path.strip_prefix(root) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let rel_name = rel.to_string_lossy().replace('\\', "/");
        zip.start_file(rel_name, options)?;

        let mut f = std::fs::File::open(&path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        zip.write_all(&buf)?;
        count += 1;
    }

    Ok(count)
}

#[get("/dashboard/download/csv")]
pub async fn download_detections_csv() -> Result<HttpResponse, Error> {
    let csv_path = "./serengeti/alerts_dataset.csv";
    let path = std::path::Path::new(csv_path);

    if !path.exists() {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "CSV file not found",
            "path": csv_path
        })));
    }

    let bytes = std::fs::read(path).map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/csv"))
        .insert_header((
            header::CONTENT_DISPOSITION,
            "attachment; filename=alerts_dataset.csv",
        ))
        .body(bytes))
}

#[get("/dashboard/devices/{device_id}/photos/download-all")]
pub async fn download_device_photos_zip(path: web::Path<u32>) -> Result<HttpResponse, Error> {
    let device_id = path.into_inner();
    let device_dir = format!("./serengeti/esp_photos/{}", device_id);
    let root = std::path::Path::new(&device_dir);

    if !root.exists() {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "No photo folder found for device",
            "device_id": device_id
        })));
    }

    let detections = collect_device_detections(device_id, usize::MAX);
    let mut sensor_by_name: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for row in detections {
        let Some(p) = row.image_path else { continue; };
        let Some(name) = std::path::Path::new(&p).file_name().and_then(|n| n.to_str()) else { continue; };
        sensor_by_name.insert(name.to_string(), row.sensor_source.to_lowercase());
    }

    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut file_count = 0usize;

    fn collect_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = entry.metadata()?;
            if meta.is_dir() {
                collect_files(&path, files)?;
            } else if meta.is_file() {
                files.push(path);
            }
        }
        Ok(())
    }

    let mut files: Vec<std::path::PathBuf> = Vec::new();
    collect_files(root, &mut files).map_err(ErrorInternalServerError)?;

    for path in files {
        let rel = match path.strip_prefix(root) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(v) => v,
            None => continue,
        };

        let sensor = sensor_by_name
            .get(filename)
            .map(|s| s.as_str())
            .unwrap_or("unknown");

        let folder = if sensor.contains("pir") {
            "pir"
        } else if sensor == "mw" || sensor.contains("microwave") {
            "mw"
        } else {
            "all"
        };

        let rel_name = rel.to_string_lossy().replace('\\', "/");
        let zip_name = format!("{}/{}", folder, rel_name);
        zip.start_file(zip_name, options)
            .map_err(ErrorInternalServerError)?;

        let mut f = std::fs::File::open(&path).map_err(ErrorInternalServerError)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(ErrorInternalServerError)?;
        zip.write_all(&buf).map_err(ErrorInternalServerError)?;
        file_count += 1;
    }

    let cursor = zip.finish().map_err(ErrorInternalServerError)?;
    let bytes = cursor.into_inner();

    if file_count == 0 {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "No photos found for device",
            "device_id": device_id
        })));
    }

    Ok(HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "application/zip"))
        .insert_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=device_{}_photos_grouped.zip", device_id),
        ))
        .body(bytes))
}

#[get("/dashboard/devices/{device_id}/detections/download-bundle")]
pub async fn download_device_detections_bundle_zip(path: web::Path<u32>) -> Result<HttpResponse, Error> {
    let device_id = path.into_inner();
    let detections = collect_device_detections(device_id, usize::MAX);

    if detections.is_empty() {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "No detections found for device",
            "device_id": device_id
        })));
    }

    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut csv_content = String::from("device_id,timestamp,sensor_source,message,image_path,image_filename\n");
    for row in &detections {
        let image_filename = row
            .image_path
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");

        csv_content.push_str(&format!(
            "{},{},{},{},{},{}\n",
            row.device_id,
            csv_cell(&row.timestamp),
            csv_cell(&row.sensor_source),
            csv_cell(&row.message),
            csv_cell(row.image_path.as_deref().unwrap_or("")),
            csv_cell(image_filename),
        ));
    }

    zip.start_file(format!("detections/device_{}_detections.csv", device_id), options)
        .map_err(ErrorInternalServerError)?;
    zip.write_all(csv_content.as_bytes())
        .map_err(ErrorInternalServerError)?;

    let mut added_images = 0usize;
    for (idx, row) in detections.iter().enumerate() {
        let Some(image_path) = &row.image_path else { continue; };
        let image_fs_path = std::path::Path::new(image_path);
        if !image_fs_path.exists() || !image_fs_path.is_file() {
            continue;
        }

        let sensor_dir = if row.sensor_source.eq_ignore_ascii_case("pir") {
            "pir"
        } else if row.sensor_source.eq_ignore_ascii_case("mw") {
            "mw"
        } else {
            "other"
        };

        let image_name = image_fs_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.jpg");
        let zip_name = format!("photos/{}/{}_{}", sensor_dir, idx + 1, image_name);

        zip.start_file(zip_name, options)
            .map_err(ErrorInternalServerError)?;
        let mut f = std::fs::File::open(image_fs_path).map_err(ErrorInternalServerError)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(ErrorInternalServerError)?;
        zip.write_all(&buf).map_err(ErrorInternalServerError)?;
        added_images += 1;
    }

    zip.start_file("README.txt", options)
        .map_err(ErrorInternalServerError)?;
    let readme = format!(
        "Device {} detection export\nRows: {}\nPhotos added: {}\nFolders: photos/pir, photos/mw, photos/other\n",
        device_id,
        detections.len(),
        added_images
    );
    zip.write_all(readme.as_bytes())
        .map_err(ErrorInternalServerError)?;

    let cursor = zip.finish().map_err(ErrorInternalServerError)?;
    let bytes = cursor.into_inner();

    Ok(HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "application/zip"))
        .insert_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=device_{}_detections_bundle.zip", device_id),
        ))
        .body(bytes))
}

#[get("/dashboard/devices/{device_id}/photos")]
pub async fn get_device_photos(path: web::Path<u32>) -> Result<HttpResponse, Error> {
    use std::fs;
    let device_id = path.into_inner();
    
    let device_photos_dir = format!("./serengeti/esp_photos/{}", device_id);
    let latest_photos_dir = format!("{}/latest", device_photos_dir);
    
    let mut photos: Vec<DevicePhoto> = Vec::new();
    
    // First, check the 'latest' folder for persistent dashboard photos
    let latest_path = std::path::Path::new(&latest_photos_dir);
    if latest_path.exists() {
        if let Ok(entries) = fs::read_dir(latest_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        let filename = entry.file_name().to_string_lossy().to_string();
                        let ext = std::path::Path::new(&filename)
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("");
                        
                        // Only include image files
                        if ["jpg", "jpeg", "png", "webp", "bmp"].contains(&ext.to_lowercase().as_str()) {
                            let modified = metadata.modified().unwrap_or(SystemTime::now());
                            photos.push(DevicePhoto {
                                filename: filename.clone(),
                                path: format!("{}/latest/{}", device_id, filename),
                                size_bytes: metadata.len(),
                                modified,
                                url: format!("/api/v1/photos/{}/latest/{}", device_id, filename),
                            });
                        }
                    }
                }
            }
        }
    }
    
    // Also check main folder for any photos currently being processed
    let photos_path = std::path::Path::new(&device_photos_dir);
    if photos_path.exists() {
        if let Ok(entries) = fs::read_dir(photos_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        let filename = entry.file_name().to_string_lossy().to_string();
                        let ext = std::path::Path::new(&filename)
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("");
                        
                        // Only include image files
                        if ["jpg", "jpeg", "png", "webp", "bmp"].contains(&ext.to_lowercase().as_str()) {
                            let modified = metadata.modified().unwrap_or(SystemTime::now());
                            photos.push(DevicePhoto {
                                filename: filename.clone(),
                                path: format!("{}/{}", device_id, filename),
                                size_bytes: metadata.len(),
                                modified,
                                url: format!("/api/v1/photos/{}/{}", device_id, filename),
                            });
                        }
                    }
                }
            }
        }
    }
    
    // Sort by modified time, newest first
    photos.sort_by(|a, b| b.modified.cmp(&a.modified));
    
    Ok(HttpResponse::Ok().json(json!({
        "device_id": device_id,
        "photos": photos,
        "count": photos.len()
    })))
}

#[get("/dashboard/devices/{device_id}/detections")]
pub async fn get_device_detections(path: web::Path<u32>) -> Result<HttpResponse, Error> {
    let device_id = path.into_inner();
    let detections = collect_device_detections(device_id, 100);

    Ok(HttpResponse::Ok().json(json!({
        "device_id": device_id,
        "detections": detections,
        "count": detections.len()
    })))
}

#[get("/dashboard/alerts")]
pub async fn get_recent_alerts() -> Result<HttpResponse, Error> {
    init_storage();
    
    unsafe {
        let mut alerts = ALERT_HISTORY.as_ref().unwrap().clone();
        alerts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        let recent_alerts: Vec<AlertSummary> = alerts.into_iter().take(20).collect();
        Ok(HttpResponse::Ok().json(recent_alerts))
    }
}

#[get("/dashboard/alerts/recent")]
pub async fn get_alerts_endpoint() -> Result<HttpResponse, Error> {
    // For now, return sample alerts - we'll implement database queries later
    let alerts: Vec<AlertSummary> = Vec::new();
    Ok(HttpResponse::Ok().json(alerts))
}

#[post("/dashboard/devices/{device_id}/acknowledge-alert/{alert_id}")]
pub async fn acknowledge_alert(path: web::Path<(u32, String)>) -> Result<HttpResponse, Error> {
    init_storage();
    let (_device_id, alert_id) = path.into_inner();
    
    unsafe {
        if let Some(alert) = ALERT_HISTORY.as_mut().unwrap().iter_mut()
            .find(|a| a.alert_id.as_ref() == Some(&alert_id)) {
            alert.acknowledged = true;
            Ok(HttpResponse::Ok().json(json!({
                "status": "acknowledged",
                "alert_id": alert_id
            })))
        } else {
            Ok(HttpResponse::NotFound().json(json!({
                "error": "Alert not found",
                "alert_id": alert_id
            })))
        }
    }
}

#[post("/dashboard/filters")]
pub async fn filter_devices(filter: web::Json<DeviceFilter>) -> Result<HttpResponse, Error> {
    init_storage();
    
    unsafe {
        let devices = DEVICE_REGISTRY.as_ref().unwrap();
        let filtered: Vec<DeviceHealth> = devices.values()
            .filter(|device| {
                // Apply status filter if provided
                if let Some(ref status) = filter.status {
                    if device.status != *status {
                        return false;
                    }
                }
                
                // Apply last-seen filter if provided
                if let Some(hours) = filter.last_seen_hours {
                    let cutoff = get_current_time()
                        - std::time::Duration::from_secs(hours as u64 * 3600);
                    if device.last_seen < cutoff {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        Ok(HttpResponse::Ok().json(filtered))
    }
}

async fn get_device_metrics() -> Vec<DeviceMetrics> {
    init_storage();
    
    unsafe {
        let devices = DEVICE_REGISTRY.as_ref().unwrap();
        let activities = DEVICE_ACTIVITIES.as_ref().unwrap();
        let alerts = ALERT_HISTORY.as_ref().unwrap();
        
        devices.keys().map(|&device_id| {
            let device_activities: Vec<DeviceActivity> = activities.iter()
                .filter(|a| a.device_id == device_id)
                .take(24)
                .cloned()
                .collect();
            
            let alerts_count = alerts.iter()
                .filter(|a| a.device_id == device_id)
                .count() as u32;
            
            DeviceMetrics {
                device_id,
                cpu_usage: Some(30.0 + (device_id as f64 * 10.0) % 50.0),
                memory_usage: Some(40.0 + (device_id as f64 * 15.0) % 40.0),
                temperature: Some(35.0 + (device_id as f64 * 5.0) % 20.0),
                data_sent_mb: 100.0 + (device_id as f64 * 25.0),
                alerts_count,
                last_24h_activity: device_activities,
            }
        }).collect()
    }
}

/// Register or update a device entry in the in-memory registry.
pub fn register_device(device: DeviceHealth) {
    init_storage();
    unsafe {
        DEVICE_REGISTRY.as_mut().unwrap().insert(device.device_id, device.clone());
    }
}

/// Register device with database persistence (async version)
pub async fn register_device_persistent(pool: &PgPool, device: DeviceHealth) {
    // Update in-memory cache first (fast)
    register_device(device.clone());
    
    // Then persist to database (async)
    if let Err(e) = db::upsert_device_health(pool, &device).await {
        eprintln!("❌ Failed to persist device {}: {}", device.device_id, e);
    }
}

/// Append a device activity entry to the activity log.
pub fn log_device_activity(activity: DeviceActivity) {
    init_storage();
    unsafe {
        DEVICE_ACTIVITIES.as_mut().unwrap().push(activity);
        // Keep only last 1000 activities
        let activities = DEVICE_ACTIVITIES.as_mut().unwrap();
        if activities.len() > 1000 {
            activities.drain(0..activities.len() - 1000);
        }
    }
}

/// Log device activity with database persistence (async version)
pub async fn log_device_activity_persistent(pool: &PgPool, activity: DeviceActivity) {
    // Update in-memory cache first (fast)
    log_device_activity(activity.clone());
    
    // Then persist to database (async)
    if let Err(e) = db::insert_device_activity(pool, &activity).await {
        eprintln!("❌ Failed to persist activity for device {}: {}", activity.device_id, e);
    }
}

/// Append an alert entry to the in-memory alert history.
pub fn log_alert(alert: AlertSummary) {
    init_storage();
    unsafe {
        ALERT_HISTORY.as_mut().unwrap().push(alert);
        // Keep only last 500 alerts
        let alerts = ALERT_HISTORY.as_mut().unwrap();
        if alerts.len() > 500 {
            alerts.drain(0..alerts.len() - 500);
        }
    }
}

/// Log alert with database persistence (async version)
pub async fn log_alert_persistent(pool: &PgPool, alert: AlertSummary) {
    // Update in-memory cache first (fast)
    log_alert(alert.clone());
    
    // Then persist to database (async)
    if let Err(e) = db::insert_alert(pool, &alert).await {
        eprintln!("❌ Failed to persist alert for device {}: {}", alert.device_id, e);
    }
}

/// Update device status (e.g., from heartbeat logic) and refresh its last_seen.
pub fn update_device_status(device_id: u32, status: HardwareStatus) {
    init_storage();
    unsafe {
        if let Some(device) = DEVICE_REGISTRY.as_mut().unwrap().get_mut(&device_id) {
            device.status = status;
            device.last_seen = get_current_time();
        }
    }
}

/// Update device status with database persistence (async version)
pub async fn update_device_status_persistent(pool: &PgPool, device_id: u32, status: HardwareStatus) {
    // Update in-memory cache first
    update_device_status(device_id, status);
    
    // Then persist to database
    unsafe {
        if let Some(device) = DEVICE_REGISTRY.as_ref().unwrap().get(&device_id) {
            if let Err(e) = db::upsert_device_health(pool, device).await {
                eprintln!("❌ Failed to persist device status for {}: {}", device_id, e);
            }
        }
    }
}

// Camera streaming endpoints

#[post("/dashboard/devices/{device_id}/camera/start")]
pub async fn start_camera_stream(
    path: web::Path<u32>,
    req_data: web::Json<StartCameraRequest>,
    mqtt_handle: web::Data<MqttBusHandle>,
) -> Result<HttpResponse, Error> {
    let device_id = path.into_inner();
    let command_id = Uuid::new_v4().to_string();
    
    // Check if device exists
    init_storage();
    unsafe {
        if !DEVICE_REGISTRY.as_ref().unwrap().contains_key(&device_id) {
            return Ok(HttpResponse::NotFound().json(CameraResponse {
                status: "error".to_string(),
                message: "Device not found".to_string(),
                stream_url: None,
                command_id: None,
            }));
        }
    }
    
    // Default camera settings
    let resolution = req_data
        .resolution
        .clone()
        .unwrap_or_else(|| "1280x720".to_string());
    let fps = req_data.fps.unwrap_or(30) as u8;
    let quality = req_data.quality.unwrap_or(70) as u8;
    let duration = req_data.duration_seconds.unwrap_or(300) as u32;
    
    // Send MQTT command to start camera
    match mqtt_handle
        .start_camera_stream(
            &device_id.to_string(),
            &command_id,
            &resolution,
            fps,
            quality,
            Some(duration),
        )
        .await
    {
        Ok(_) => {
            let stream_url = format!("/ws/camera/{}", device_id);
            
            Ok(HttpResponse::Ok().json(CameraResponse {
                status: "success".to_string(),
                message: "Camera stream start command sent".to_string(),
                stream_url: Some(stream_url),
                command_id: Some(command_id),
            }))
        }
        Err(e) => {
            Ok(HttpResponse::InternalServerError().json(CameraResponse {
                status: "error".to_string(),
                message: format!("Failed to send camera command: {}", e),
                stream_url: None,
                command_id: None,
            }))
        }
    }
}

#[post("/dashboard/devices/{device_id}/camera/stop")]
pub async fn stop_camera_stream(
    path: web::Path<u32>,
    mqtt_handle: web::Data<MqttBusHandle>,
) -> Result<HttpResponse, Error> {
    let device_id = path.into_inner();
    let command_id = Uuid::new_v4().to_string();
    
    // Check if device exists
    init_storage();
    unsafe {
        if !DEVICE_REGISTRY.as_ref().unwrap().contains_key(&device_id) {
            return Ok(HttpResponse::NotFound().json(CameraResponse {
                status: "error".to_string(),
                message: "Device not found".to_string(),
                stream_url: None,
                command_id: None,
            }));
        }
    }
    
    // Send MQTT command to stop camera
    match mqtt_handle
        .stop_camera_stream(&device_id.to_string(), &command_id)
        .await
    {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(CameraResponse {
                status: "success".to_string(),
                message: "Camera stream stop command sent".to_string(),
                stream_url: None,
                command_id: Some(command_id),
            }))
        }
        Err(e) => {
            Ok(HttpResponse::InternalServerError().json(CameraResponse {
                status: "error".to_string(),
                message: format!("Failed to send stop command: {}", e),
                stream_url: None,
                command_id: None,
            }))
        }
    }
}

#[get("/dashboard/devices/{device_id}/camera/status")]
pub async fn get_camera_status(path: web::Path<u32>) -> Result<HttpResponse, Error> {
    let device_id = path.into_inner();
    
    // Check if device exists
    init_storage();
    unsafe {
        if !DEVICE_REGISTRY.as_ref().unwrap().contains_key(&device_id) {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Device not found",
                "device_id": device_id
            })));
        }
    }
    
    // For now return basic status - this could be enhanced to track actual streaming state
    Ok(HttpResponse::Ok().json(json!({
        "device_id": device_id,
        "camera_available": true,
        "streaming": false, // This could be tracked via MQTT telemetry
        "stream_url": format!("/ws/camera/{}", device_id),
        "capabilities": {
            "resolutions": ["640x480", "1280x720", "1920x1080"],
            "max_fps": 30,
            "formats": ["MJPEG", "H264"]
        }
    })))
}

/// Restart device via MQTT
#[post("/devices/{device_id}/restart")]
pub async fn restart_device(
    path: web::Path<String>,
    mqtt_handle: web::Data<MqttBusHandle>,
) -> Result<HttpResponse, Error> {
    let device_id_str = path.into_inner();
    let command_id = Uuid::new_v4().to_string();
    
    // Parse device_id as u32 for registry check
    let device_id: u32 = match device_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": "Invalid device ID format",
                "device_id": device_id_str
            })));
        }
    };
    
    // Check if device exists
    init_storage();
    unsafe {
        if !DEVICE_REGISTRY.as_ref().unwrap().contains_key(&device_id) {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Device not found",
                "device_id": device_id
            })));
        }
    }
    
    // Send restart command via MQTT
    match mqtt_handle.restart_device(&device_id_str, &command_id).await {
        Ok(_) => {
            println!("✅ Restart command sent to device {} via MQTT", device_id);
            Ok(HttpResponse::Ok().json(json!({
                "status": "success",
                "message": "Restart command sent",
                "device_id": device_id,
                "command_id": command_id
            })))
        }
        Err(e) => {
            eprintln!("❌ Failed to send restart command: {}", e);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to send restart command",
                "details": e.to_string()
            })))
        }
    }
}
