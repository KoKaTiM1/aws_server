use crate::services::circuit_breaker::CircuitBreaker;
use crate::services::ws::HardwareWebSocket;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::time::Duration;

// Initialize circuit breaker with 30s reset timeout and 5 failure threshold
lazy_static::lazy_static! {
    static ref CIRCUIT_BREAKER: CircuitBreaker = CircuitBreaker::new(
        Duration::from_secs(30),
        5
    );
}

pub async fn hardware_ws(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    // Check circuit breaker state
    if !CIRCUIT_BREAKER.can_execute().await {
        tracing::error!("Circuit breaker open - temporarily refusing new WebSocket connections");
        return Err(actix_web::error::ErrorServiceUnavailable(
            "Service temporarily unavailable",
        ));
    }

    // Extract and validate hardware ID
    let hardware_id = match req
        .headers()
        .get("X-Hardware-ID")
        .and_then(|id| id.to_str().ok())
    {
        Some(id) if !id.is_empty() => id,
        _ => {
            tracing::warn!(
                "Invalid hardware ID from IP: {}",
                req.connection_info()
                    .realip_remote_addr()
                    .unwrap_or("unknown")
            );
            return Err(actix_web::error::ErrorBadRequest("Invalid hardware ID"));
        }
    };

    // Verify protocol version
    match req.headers().get("Sec-WebSocket-Protocol") {
        Some(protocol) if protocol == "eye-dar-v1" => (),
        _ => {
            tracing::warn!("Invalid protocol version from hardware ID: {}", hardware_id);
            return Err(actix_web::error::ErrorBadRequest(
                "Unsupported protocol version",
            ));
        }
    }

    // Create WebSocket connection
    let ws = HardwareWebSocket::new(hardware_id.to_string());
    match ws::start(ws, &req, stream) {
        Ok(resp) => {
            CIRCUIT_BREAKER.record_success().await;
            tracing::info!(
                "WebSocket connection established for hardware ID: {}",
                hardware_id
            );
            Ok(resp)
        }
        Err(e) => {
            CIRCUIT_BREAKER.record_failure().await;
            tracing::error!(
                "Failed to establish WebSocket connection for hardware ID: {} - Error: {}",
                hardware_id,
                e
            );
            Err(e)
        }
    }
}
