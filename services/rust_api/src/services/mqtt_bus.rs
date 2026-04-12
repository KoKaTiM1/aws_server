//! MQTT bus: publishes commands to devices and ingests device events/pongs.
//! Topics (simple, per-device):
//!   cmd/<device_id>   -- server → device (JSON commands)
//!   tele/<device_id>  -- device → server (JSON events: {"type":"pong", ...}, {"type":"init", ...})

use std::time::Duration; // Remove sync::Arc
use tokio::{select, sync::mpsc, time::sleep};
use rumqttc::{AsyncClient, Event, MqttOptions, QoS, Incoming}; // Remove EventLoop, Packet
use serde_json::json;
use crate::services::heartbeat::{HeartbeatRegistry, upsert_device};

#[derive(Clone)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_id_prefix: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keepalive: Duration,
}

#[derive(Debug)]
pub enum MqttCmd {
    Ping { device_id: String, cmd_id: String },
    // Camera streaming commands
    StartCameraStream { 
        device_id: String, 
        cmd_id: String, 
        resolution: String,
        fps: u8,
        quality: u8,
        duration_seconds: Option<u32>,
    },
    StopCameraStream { 
        device_id: String, 
        cmd_id: String 
    },
    RestartDevice {
        device_id: String,
        cmd_id: String
    },
    // Future (Steps 4–5):
    // SetInterval { device_id: String, pct: u8, once: bool, cmd_id: String },
    // RestartPing { device_id: String, cmd_id: String },
}

#[derive(Clone)]
pub struct MqttBusHandle {
    tx: mpsc::Sender<MqttCmd>,
}
impl MqttBusHandle {
    pub async fn ping(&self, device_id: &str, cmd_id: &str) -> Result<(), mpsc::error::SendError<MqttCmd>> {
        self.tx.send(MqttCmd::Ping { device_id: device_id.to_string(), cmd_id: cmd_id.to_string() }).await
    }
    
    pub async fn start_camera_stream(
        &self, 
        device_id: &str, 
        cmd_id: &str,
        resolution: &str,
        fps: u8,
        quality: u8,
        duration_seconds: Option<u32>,
    ) -> Result<(), mpsc::error::SendError<MqttCmd>> {
        self.tx.send(MqttCmd::StartCameraStream {
            device_id: device_id.to_string(),
            cmd_id: cmd_id.to_string(),
            resolution: resolution.to_string(),
            fps,
            quality,
            duration_seconds,
        }).await
    }
    
    pub async fn stop_camera_stream(&self, device_id: &str, cmd_id: &str) -> Result<(), mpsc::error::SendError<MqttCmd>> {
        self.tx.send(MqttCmd::StopCameraStream {
            device_id: device_id.to_string(),
            cmd_id: cmd_id.to_string(),
        }).await
    }
    
    pub async fn restart_device(&self, device_id: &str, cmd_id: &str) -> Result<(), mpsc::error::SendError<MqttCmd>> {
        self.tx.send(MqttCmd::RestartDevice {
            device_id: device_id.to_string(),
            cmd_id: cmd_id.to_string(),
        }).await
    }
}

pub fn spawn_mqtt_bus(cfg: MqttConfig, reg: HeartbeatRegistry) -> MqttBusHandle {
    let (tx, mut rx) = mpsc::channel::<MqttCmd>(256);

    tokio::spawn(async move {
        let client_id = format!("{}-{}", cfg.client_id_prefix, uuid::Uuid::new_v4());
        let mut opts = MqttOptions::new(client_id, cfg.host.clone(), cfg.port);
        opts.set_keep_alive(cfg.keepalive);
        
        // Enable TLS for HiveMQ Cloud
        opts.set_transport(rumqttc::Transport::tls_with_default_config());
        
        if let Some(user) = &cfg.username {
            opts.set_credentials(user.clone(), cfg.password.clone().unwrap_or_default());
        }

        let (client, mut eventloop) = AsyncClient::new(opts, 32);

        // Connect and subscribe
        loop {
            // Retry connect + subscribe until success
            if let Err(e) = client.subscribe("tele/+", QoS::AtLeastOnce).await {
                eprintln!("[mqtt] subscribe tele/+ failed: {e}. Retrying in 2s...");
                sleep(Duration::from_secs(2)).await;
                continue;
            }
            break;
        }
        println!("[mqtt] connected; subscribed to tele/+");

        // Event loop: drive MQTT + publish commands + ingest device telemetry
        loop {
            select! {
                maybe_cmd = rx.recv() => {
                    if let Some(cmd) = maybe_cmd {
                        match cmd {
                            MqttCmd::Ping { device_id, cmd_id } => {
                                let topic = format!("cmd/{device_id}");
                                let payload = json!({"cmd":"ping", "cmd_id": cmd_id}).to_string();
                                if let Err(e) = client.publish(topic, QoS::AtLeastOnce, false, payload).await {
                                    eprintln!("[mqtt] publish ping failed: {e}");
                                }
                            }
                            MqttCmd::StartCameraStream { device_id, cmd_id, resolution, fps, quality, duration_seconds } => {
                                let topic = format!("cmd/{device_id}");
                                let payload = json!({
                                    "cmd": "start_camera_stream",
                                    "cmd_id": cmd_id,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                    "parameters": {
                                        "resolution": resolution,
                                        "fps": fps,
                                        "quality": quality,
                                        "duration_seconds": duration_seconds,
                                        "stream_endpoint": format!("ws://100.64.0.1:8080/api/v1/ws/camera/{}", device_id)
                                    },
                                    "timeout_seconds": 60
                                }).to_string();
                                
                                if let Err(e) = client.publish(topic, QoS::AtLeastOnce, false, payload).await {
                                    eprintln!("[mqtt] publish start_camera_stream failed: {e}");
                                } else {
                                    println!("[mqtt] Started camera stream for {} ({}@{}fps)", device_id, resolution, fps);
                                }
                            }
                            MqttCmd::StopCameraStream { device_id, cmd_id } => {
                                let topic = format!("cmd/{device_id}");
                                let payload = json!({
                                    "cmd": "stop_camera_stream",
                                    "cmd_id": cmd_id,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                    "timeout_seconds": 10
                                }).to_string();
                                
                                if let Err(e) = client.publish(topic, QoS::AtLeastOnce, false, payload).await {
                                    eprintln!("[mqtt] publish stop_camera_stream failed: {e}");
                                } else {
                                    println!("[mqtt] Stopped camera stream for {}", device_id);
                                }
                            }
                            MqttCmd::RestartDevice { device_id, cmd_id } => {
                                let topic = format!("cmd/{device_id}");
                                let payload = json!({
                                    "cmd": "restart",
                                    "cmd_id": cmd_id
                                }).to_string();
                                
                                if let Err(e) = client.publish(topic, QoS::AtLeastOnce, false, payload).await {
                                    eprintln!("[mqtt] publish restart failed: {e}");
                                } else {
                                    println!("[mqtt] Restart command sent to device {} (cmd_id: {})", device_id, cmd_id);
                                }
                            }
                        }
                    } else {
                        break; // sender dropped
                    }
                }
                ev = eventloop.poll() => {
                    match ev {
                        Ok(Event::Incoming(Incoming::Publish(p))) => {
                            // Expect topics: tele/<device_id>
                            let topic = p.topic.as_str();
                            let payload_str = String::from_utf8_lossy(&p.payload);
                            if let Some(device_id) = topic.strip_prefix("tele/") {
                                // Process device event JSON
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload_str) {
                                    // On any 'pong' or 'init', mark device as seen
                                    let is_pong = v.get("type").and_then(|x| x.as_str()) == Some("pong")
                                        || v.get("status").and_then(|x| x.as_str()) == Some("pong");
                                    let is_init = v.get("type").and_then(|x| x.as_str()) == Some("init");

                                    if is_pong || is_init {
                                        // No LAN address in MQTT path; just update last_seen
                                        upsert_device(&reg, device_id, None, None).await;
                                        println!("[mqtt] updated last-seen for {device_id} via {}", if is_pong {"pong"} else {"init"});
                                    }
                                }
                            }
                        }
                        Ok(_) => {} // ignore pings/acks
                        Err(e) => {
                            eprintln!("[mqtt] eventloop error: {e}. Reconnecting in 2s...");
                            sleep(Duration::from_secs(2)).await;
                            // In a full impl you'd rebuild client & resubscribe here.
                        }
                    }
                }
            }
        }
    });

    MqttBusHandle { tx }
}

pub fn create_hivemq_config() -> MqttConfig {
    MqttConfig {
        host: std::env::var("MQTT_HOST")
            .expect("MQTT_HOST environment variable must be set"),
        port: std::env::var("MQTT_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8883),
        client_id_prefix: "server".to_string(),
        username: Some(std::env::var("MQTT_USERNAME")
            .expect("MQTT_USERNAME environment variable must be set")),
        password: Some(std::env::var("MQTT_PASSWORD")
            .expect("MQTT_PASSWORD environment variable must be set")),
        keepalive: Duration::from_secs(60),
    }
}
