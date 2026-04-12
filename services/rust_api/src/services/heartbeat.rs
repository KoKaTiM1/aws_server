use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time};
use chrono::{DateTime, Utc};
use crate::services::mqtt_bus::MqttBusHandle;
use crate::routes::dashboard::update_device_status;
use crate::models::hardware::HardwareStatus;
use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub last_seen_utc: DateTime<Utc>,
    pub addr: Option<SocketAddr>, // e.g., 10.0.0.9:8088
}

#[derive(Clone)]
pub struct HeartbeatRegistry(pub Arc<RwLock<HashMap<String, DeviceInfo>>>);

impl HeartbeatRegistry {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
}

impl Default for HeartbeatRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn upsert_device(
    reg: &HeartbeatRegistry,
    device_id: &str,
    maybe_ip: Option<&str>,
    maybe_port: Option<u16>,
) {
    let mut map = reg.0.write().await;
    let addr = match (maybe_ip, maybe_port) {
        (Some(ip), Some(port)) => SocketAddr::from_str(&format!("{}:{}", ip, port)).ok(),
        _ => None,
    };
    map.insert(
        device_id.to_string(),
        DeviceInfo { last_seen_utc: Utc::now(), addr },
    );
}

/// Watchdog: for devices silent > `stale_after`, publish an MQTT "ping" command.
/// The device should publish a "pong" to tele/<device_id> which updates last_seen.
pub fn spawn_watchdog_mqtt(
    reg: HeartbeatRegistry,
    bus: MqttBusHandle,
    stale_after: Duration,
    probe_every: Duration,
) {
    tokio::spawn(async move {
        loop {
            time::sleep(probe_every).await;
            let snapshot = { reg.0.read().await.clone() };
            let now = Utc::now();

            for (device_id, info) in snapshot {
                let silent = now.signed_duration_since(info.last_seen_utc).to_std().unwrap_or_default() > stale_after;
                if !silent { continue; }

                let cmd_id = now.timestamp_millis().to_string();
                if let Err(e) = bus.ping(&device_id, &cmd_id).await {
                    eprintln!("[watchdog] mqtt ping enqueue failed for {device_id}: {e}");
                } else {
                    println!("[watchdog] mqtt ping → cmd/{device_id} (cmd_id={cmd_id})");
                }
            }
        }
    });
}

/// Offline detector: periodically checks all registered devices and marks them as offline
/// if they haven't been seen in `offline_threshold` duration.
pub fn spawn_offline_detector(pool: PgPool, offline_threshold: Duration, check_interval: Duration) {
    tokio::spawn(async move {
        loop {
            time::sleep(check_interval).await;
            
            // Query database for all devices
            match crate::db::load_all_devices(&pool).await {
                Ok(devices) => {
                    let now = std::time::SystemTime::now();
                    
                    for device in devices {
                        // Check if device is stale based on last_seen timestamp
                        if let Ok(elapsed) = now.duration_since(device.last_seen) {
                            if elapsed > offline_threshold && !matches!(device.status, HardwareStatus::Offline) {
                                let seconds = elapsed.as_secs();
                                
                                // Update in-memory and persist to database
                                crate::routes::dashboard::update_device_status_persistent(&pool, device.device_id, HardwareStatus::Offline).await;
                                
                                tracing::warn!("[offline-detector] ⚠️ Device {} marked as OFFLINE (last seen {} seconds ago)", device.device_id, seconds);
                            }
                        }
                    }
                }
                Err(e) => eprintln!("[offline-detector] Failed to load devices: {}", e),
            }
        }
    });
}

// pub fn spawn_watchdog(
//     reg: HeartbeatRegistry,
//     stale_after: Duration,
//     probe_every: Duration,
//     timeout: Duration,
// ) {
//     tokio::spawn(async move {
//         let client = reqwest::Client::builder()
//             .timeout(timeout)
//             .redirect(reqwest::redirect::Policy::none())
//             .use_rustls_tls()
//             .build()
//             .expect("reqwest client");

//         loop {
//             time::sleep(probe_every).await;

//             let snapshot = { reg.0.read().await.clone() };
//             let now = Utc::now();

//             for (device_id, info) in snapshot {
//                 if now.signed_duration_since(info.last_seen_utc).to_std().unwrap_or_default() <= stale_after {
//                     continue;
//                 }
//                 if let Some(addr) = info.addr {
//                     let url = format!("http://{}/ping", addr);
//                     match client.get(&url).send().await {
//                         Ok(resp) if resp.status().is_success() => {
//                             upsert_device(&reg, &device_id, Some(&addr.ip().to_string()), Some(addr.port())).await;
//                             println!("[watchdog] {} alive via probe {}", device_id, url);
//                         }
//                         Ok(resp) => {
//                             println!("[watchdog] {} probe {} → HTTP {}", device_id, url, resp.status());
//                         }
//                         Err(err) => {
//                             println!("[watchdog] {} probe {} failed: {}", device_id, url, err);
//                         }
//                     }
//                 } else {
//                     println!("[watchdog] {} has no reachable addr; skipping probe", device_id);
//                 }
//             }
//         }
//     });
// }
