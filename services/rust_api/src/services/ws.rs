use actix::prelude::*;
use actix_web_actors::ws::{Message, ProtocolError, WebsocketContext};
use serde_json::Value;
use std::time::{Duration, Instant};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct HardwareWebSocket {
    hardware_id: String,
    hb: Instant,
}

impl HardwareWebSocket {
    pub fn new(hardware_id: String) -> Self {
        Self {
            hardware_id,
            hb: Instant::now(),
        }
    }

    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                tracing::warn!("Hardware WebSocket Client timeout: {}", act.hardware_id);
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}

impl Actor for HardwareWebSocket {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
        tracing::info!(
            "Hardware WebSocket connection started: {}",
            self.hardware_id
        );
    }
}

impl StreamHandler<Result<Message, ProtocolError>> for HardwareWebSocket {
    fn handle(&mut self, msg: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(Message::Text(text)) => {
                if let Ok(data) = serde_json::from_str::<Value>(&text) {
                    tracing::info!(
                        "Received data from hardware {}: {:?}",
                        self.hardware_id,
                        data
                    );
                    // Echo back confirmation
                    ctx.text(
                        serde_json::json!({
                            "status": "received",
                            "hardware_id": self.hardware_id
                        })
                        .to_string(),
                    );
                }
            }
            Ok(Message::Binary(bin)) => {
                tracing::info!(
                    "Received binary data from hardware {}: {} bytes",
                    self.hardware_id,
                    bin.len()
                );
                ctx.binary(bin);
            }
            Ok(Message::Close(reason)) => {
                tracing::info!(
                    "Hardware WebSocket closing: {} (reason: {:?})",
                    self.hardware_id,
                    reason
                );
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}
