// src/services/mqtt_monitor.rs
//
// Extended for Step 6:
// - Track restart attempts
// - Verify device sends telemetry after restart
// - Escalate if no init seen within grace period

use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time;
use chrono::Utc;
use std::env;
use serde_json::Value;
use lazy_static::lazy_static;

// Shared device settings cache
lazy_static! {
    static ref DEVICE_SETTINGS: Arc<Mutex<HashMap<u32, DeviceSettings>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Clone, Debug)]
struct DeviceSettings {
    mqtt_enabled: bool,
    heartbeat_enabled: bool,
    last_updated: Instant,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self {
            mqtt_enabled: true,
            heartbeat_enabled: true,
            last_updated: Instant::now(),
        }
    }
}

fn get_timeout_config() -> (Duration, Duration, f32, Duration) {
    let ping_interval = env::var("PING_INTERVAL_SEC").unwrap_or_else(|_| "10".to_string());
    let timeout = env::var("TIMEOUT_SEC").unwrap_or_else(|_| "30".to_string());
    let fast_factor = env::var("FAST_INTERVAL_FACTOR").unwrap_or_else(|_| "0.05".to_string());
    let restart_wait = env::var("RESTART_WAIT_SEC").unwrap_or_else(|_| "60".to_string());

    (
        Duration::from_secs(ping_interval.parse().unwrap_or(10)),
        Duration::from_secs(timeout.parse().unwrap_or(30)),
        fast_factor.parse().unwrap_or(0.05),
        Duration::from_secs(restart_wait.parse().unwrap_or(60)),
    )
}

// Check device settings from cache (fetched via REST API)
fn should_process_message(device_id_str: &str, msg_type: &str) -> bool {
    // Parse device_id as u32
    let device_id: u32 = match device_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            eprintln!("⚠️  Invalid device ID format: {}", device_id_str);
            return true; // Process message if ID is invalid (fail open)
        }
    };

    let settings = DEVICE_SETTINGS.lock().unwrap();
    
    if let Some(device_settings) = settings.get(&device_id) {
        // If MQTT is disabled for this device, ignore all messages
        if !device_settings.mqtt_enabled {
            println!("🚫 Ignoring MQTT message from {} (MQTT disabled)", device_id);
            return false;
        }

        // If this is a ping/pong and heartbeats are disabled, ignore
        if (msg_type == "ping" || msg_type == "pong") && !device_settings.heartbeat_enabled {
            println!("🚫 Ignoring heartbeat from {} (heartbeats disabled)", device_id);
            return false;
        }
    }

    // Default: process the message (device not in cache or settings allow it)
    true
}

// Fetch device settings from the REST API
async fn fetch_device_settings(device_id: u32) -> Option<DeviceSettings> {
    let api_url = format!("http://rust_api:8080/api/v1/dashboard/devices/{}", device_id);
    
    match reqwest::get(&api_url).await {
        Ok(response) if response.status().is_success() => {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                Some(DeviceSettings {
                    mqtt_enabled: json.get("mqtt_enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                    heartbeat_enabled: json.get("heartbeat_enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                    last_updated: Instant::now(),
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

// Periodically refresh device settings from API
async fn refresh_device_settings_loop() {
    let mut interval = time::interval(Duration::from_secs(30)); // Refresh every 30 seconds
    
    loop {
        interval.tick().await;
        
        // Get list of device IDs we're tracking
        let device_ids: Vec<u32> = {
            let settings = DEVICE_SETTINGS.lock().unwrap();
            settings.keys().copied().collect()
        };
        
        // Refresh settings for each device
        for device_id in device_ids {
            if let Some(new_settings) = fetch_device_settings(device_id).await {
                DEVICE_SETTINGS.lock().unwrap().insert(device_id, new_settings);
            }
        }
    }
}


#[tokio::main]
async fn main() {
    // Use HiveMQ Cloud configuration from environment variables
    let mqtt_host = env::var("MQTT_HOST")
        .expect("MQTT_HOST environment variable must be set");
    let mqtt_port: u16 = env::var("MQTT_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8883);
    let mqtt_username = env::var("MQTT_USERNAME")
        .expect("MQTT_USERNAME environment variable must be set");
    let mqtt_password = env::var("MQTT_PASSWORD")
        .expect("MQTT_PASSWORD environment variable must be set");
    
    let mut mqttoptions = MqttOptions::new("mqtt-monitor", mqtt_host, mqtt_port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    
    // Set up TLS for HiveMQ Cloud
    mqttoptions.set_transport(rumqttc::Transport::tls_with_default_config());
    mqttoptions.set_credentials(mqtt_username, mqtt_password);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Subscribe to all telemetry
    client.subscribe("tele/#", QoS::AtMostOnce).await.unwrap();
    println!("✓ MQTT monitor started, subscribed to tele/# on HiveMQ Cloud");

    // Start background task to refresh device settings from API
    tokio::spawn(refresh_device_settings_loop());
    println!("✓ Device settings refresh task started");

    // Use Arc<Mutex<>> for shared state between tasks
    let last_seen: Arc<Mutex<HashMap<String, Instant>>> = Arc::new(Mutex::new(HashMap::new()));
    let fast_interval_tracker: Arc<Mutex<HashMap<String, Instant>>> = Arc::new(Mutex::new(HashMap::new()));
    let restart_tracker: Arc<Mutex<HashMap<String, Instant>>> = Arc::new(Mutex::new(HashMap::new()));


    // Clone for background task
    let client_clone = client.clone();
    let last_seen_clone = last_seen.clone();
    let fast_interval_tracker_clone = fast_interval_tracker.clone();
    let restart_tracker_clone = restart_tracker.clone();

    
    tokio::spawn(async move {
        // Use configured ping interval for check frequency
        let (ping_interval, timeout, _fast_factor, restart_wait) = get_timeout_config();
        let mut interval = time::interval(ping_interval);
        loop {
            interval.tick().await;

            // --- Timeout check (send set_interval) ---
            let devices_to_handle: Vec<String> = {
                let last_seen_guard = last_seen_clone.lock().unwrap();
                last_seen_guard.iter()
                    .filter_map(|(device_id, ts)| {
                        if ts.elapsed() > timeout {
                            Some(device_id.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            };

            for device_id in devices_to_handle {
                println!("⚠ Device {} missed ping, sending set_interval...", device_id);

                let temp_interval_ms = (ping_interval.as_millis() as f32 * _fast_factor) as u64;
                let cmd = format!(
                    r#"{{"cmd":"set_interval","value":{},"cmd_id":"{}-{}"}}"#,
                    temp_interval_ms,
                    device_id,
                    Utc::now().timestamp()
                );

                let topic = format!("cmd/{}", device_id);
                let _ = client_clone.publish(topic, QoS::AtLeastOnce, false, cmd).await;

                // Track in fast_interval
                fast_interval_tracker_clone.lock().unwrap().insert(device_id.clone(), Instant::now());
            }

            // --- Post-set_interval verification ---
            let devices_to_restart: Vec<String> = {
                let tracker_guard = fast_interval_tracker_clone.lock().unwrap();
                tracker_guard.iter()
                    .filter_map(|(device_id, ts)| {
                        if ts.elapsed() > restart_wait {
                            Some(device_id.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            };

            for device_id in devices_to_restart {
                println!("⚠ Device {} did not respond to set_interval, sending restart...", device_id);

                let cmd = format!(
                    r#"{{"cmd":"restart","cmd_id":"{}-{}"}}"#,
                    device_id,
                    Utc::now().timestamp()
                );

                let topic = format!("cmd/{}", device_id);
                let _ = client_clone.publish(topic, QoS::AtLeastOnce, false, cmd).await;

                // Move device to restart tracker
                fast_interval_tracker_clone.lock().unwrap().remove(&device_id);
                restart_tracker_clone.lock().unwrap().insert(device_id.clone(), Instant::now());
            }

            // --- Post-restart verification (final escalation) ---
            let devices_to_escalate: Vec<String> = {
                let restart_tracker_guard = restart_tracker_clone.lock().unwrap();
                restart_tracker_guard.iter()
                    .filter_map(|(device_id, ts)| {
                        if ts.elapsed() > restart_wait {
                            Some(device_id.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            };

            for device_id in devices_to_escalate {
                println!("🚨 Device {} failed to re-init after restart. Escalate technician flag!", device_id);
                restart_tracker_clone.lock().unwrap().remove(&device_id);
            }

        }
    });

    // Process MQTT events
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(incoming)) => {
                if let rumqttc::Packet::Publish(p) = incoming {
                    let topic = p.topic.clone();
                    let payload = String::from_utf8_lossy(&p.payload);

                    if topic.starts_with("tele/") {
                        let device_id_str = topic.trim_start_matches("tele/").to_string();
                        
                        // Parse device_id and add to settings cache if new
                        if let Ok(device_id) = device_id_str.parse::<u32>() {
                            let mut settings = DEVICE_SETTINGS.lock().unwrap();
                            if !settings.contains_key(&device_id) {
                                // New device discovered, fetch its settings
                                drop(settings); // Release lock before async operation
                                if let Some(new_settings) = fetch_device_settings(device_id).await {
                                    DEVICE_SETTINGS.lock().unwrap().insert(device_id, new_settings);
                                    println!("✓ Discovered new device {} and cached settings", device_id);
                                } else {
                                    // Use default settings if API fetch fails
                                    DEVICE_SETTINGS.lock().unwrap().insert(device_id, DeviceSettings::default());
                                }
                            }
                        }
                        
                        // Parse message type
                        let msg_type = if let Ok(v) = serde_json::from_str::<Value>(&payload) {
                            v.get("type").and_then(|x| x.as_str()).unwrap_or("unknown").to_string()
                        } else {
                            "unknown".to_string()
                        };

                        // Check if we should process this message based on device settings
                        if !should_process_message(&device_id_str, &msg_type) {
                            continue; // Skip this message
                        }

                        // Update last seen timestamp
                        last_seen.lock().unwrap().insert(device_id_str.clone(), Instant::now());

                        // Clear fast_interval tracker if device responded
                        if fast_interval_tracker.lock().unwrap().contains_key(&device_id_str) {
                            println!("✅ Device {} responded after set_interval", device_id_str);
                            fast_interval_tracker.lock().unwrap().remove(&device_id_str);
                        }

                        // Clear restart tracker if device responded after restart
                        if restart_tracker.lock().unwrap().contains_key(&device_id_str) {
                            println!("✅ Device {} sent init telemetry after restart", device_id_str);
                            restart_tracker.lock().unwrap().remove(&device_id_str);
                        }


                        println!("↪ Telemetry from {}: {}", device_id_str, payload);

                        // Try to decode latency_test messages and compute end-to-end latency
                        if let Ok(v) = serde_json::from_str::<Value>(&payload) {
                            // Ensure this is a latency_test message
                            if v.get("type").and_then(|x| x.as_str()) == Some("latency_test") {
                                // Extract the send time (in milliseconds since Unix epoch) from the device
                                if let Some(sent_ms) = v.get("unix_timestamp_ms").and_then(|x| x.as_i64()) {
                                     // Current server time in milliseconds since Unix epoch (UTC)
                                    let now_ms = Utc::now().timestamp_millis();

                                     // Latency = server receive time - device send time
                                    let latency_ms = now_ms - sent_ms;

                                    // Optional: read test_id if present
                                    let test_id = v.get("test_id")
                                        .and_then(|x| x.as_i64())
                                        .unwrap_or(-1);

                                    println!(
                                        "[latency] device_id={} test_id={} latency={} ms (server_now_ms={} device_sent_ms={})",
                                        device_id_str,
                                        test_id,
                                        latency_ms,
                                        now_ms,
                                        sent_ms
                                    );
                                 }   
                             }
                        }

                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("MQTT error: {:?}", e);
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}