use actix_web::{post, web, HttpResponse, HttpRequest}; // + HttpRequest
use serde::Deserialize;
use serde_json::json;
use crate::services::heartbeat::{HeartbeatRegistry, upsert_device}; // Use crate:: instead of rust_api::

// Request body DTO for POST /ping
#[derive(Deserialize)]
pub struct PingRequest {
    pub device_id: String,
    pub timestamp: String,
    pub lan_ip: Option<String>, // e.g., "10.0.0.9"
    pub port:   Option<u16>,    // e.g., 8088
}

#[post("/ping")]
pub async fn receive_ping(
    req: HttpRequest,
    registry: web::Data<HeartbeatRegistry>,
    payload: web::Json<PingRequest>,
) -> HttpResponse {
    // Update last-seen and contact point
    upsert_device(&registry, &payload.device_id, payload.lan_ip.as_deref(), payload.port).await;

    let peer = payload
        .lan_ip
        .clone()
        .or_else(|| req.peer_addr().map(|sa| sa.ip().to_string()))
        .unwrap_or_else(|| "<unknown>".to_string());

    println!("Ping from {} at {} (ip_for_log={})",
        payload.device_id, payload.timestamp, peer);

    HttpResponse::Ok().json(json!({
        "status": "pong",
        "device_id": payload.device_id,
        "timestamp": payload.timestamp,
    }))
}

// Registrar unchanged
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(receive_ping);
}
