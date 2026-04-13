use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use crate::models::hardware::HardwareStatus;

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
