use actix::prelude::*;
use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_web_actors::ws;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

// Global registry to track active camera streams
lazy_static::lazy_static! {
    static ref CAMERA_STREAMS: Arc<Mutex<HashMap<String, Vec<Addr<CameraWebSocket>>>>> = 
        Arc::new(Mutex::new(HashMap::new()));
}

pub struct CameraWebSocket {
    device_id: String,
}

impl Actor for CameraWebSocket {
    type Context = ws::WebsocketContext<Self>;
    
    fn started(&mut self, ctx: &mut Self::Context) {
        println!("[camera-ws] Client connected for device: {}", self.device_id);
        
        // Register this WebSocket for the device
        let mut streams = CAMERA_STREAMS.lock().unwrap();
        streams.entry(self.device_id.clone())
            .or_insert_with(Vec::new)
            .push(ctx.address());
    }
    
    fn stopped(&mut self, ctx: &mut Self::Context) {
        println!("[camera-ws] Client disconnected from device: {}", self.device_id);
        
        // Remove this WebSocket from the registry
        let mut streams = CAMERA_STREAMS.lock().unwrap();
        if let Some(device_streams) = streams.get_mut(&self.device_id) {
            device_streams.retain(|addr| addr != &ctx.address());
            if device_streams.is_empty() {
                streams.remove(&self.device_id);
            }
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for CameraWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                println!("[camera-ws] Received text from {}: {}", self.device_id, text);
                
                // Handle client commands (like stream control)
                if let Ok(cmd) = serde_json::from_str::<CameraCommand>(&text) {
                    self.handle_command(cmd, ctx);
                }
            }
            Ok(ws::Message::Binary(data)) => {
                // This is a camera frame from the ESP32 device
                println!("[camera-ws] Received frame from {}: {} bytes", self.device_id, data.len());
                
                // Broadcast this frame to all connected dashboard clients
                broadcast_camera_frame(&self.device_id, data.to_vec(), "jpeg");
            }
            Ok(ws::Message::Close(reason)) => {
                println!("[camera-ws] Connection closed for {}: {:?}", self.device_id, reason);
                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }
    }
}

impl CameraWebSocket {
    fn handle_command(&self, cmd: CameraCommand, _ctx: &mut ws::WebsocketContext<Self>) {
        match cmd {
            CameraCommand::RequestFrame => {
                println!("[camera-ws] Frame requested for device {}", self.device_id);
                // Could send a command to ESP32 to capture a frame
            }
            CameraCommand::SetQuality { quality } => {
                println!("[camera-ws] Quality change requested for {}: {}", self.device_id, quality);
                // Could send quality adjustment command to ESP32
            }
        }
    }
}

// Message type for sending camera frames to dashboard clients
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct CameraFrame {
    pub device_id: String,
    pub frame_data: Vec<u8>,
    pub frame_type: String, // "jpeg", "h264", etc.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Handler<CameraFrame> for CameraWebSocket {
    type Result = ();
    
    fn handle(&mut self, msg: CameraFrame, ctx: &mut Self::Context) {
        // Only send frames for this device
        if msg.device_id == self.device_id {
            ctx.binary(msg.frame_data);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CameraCommand {
    RequestFrame,
    SetQuality { quality: u8 },
}

/// WebSocket endpoint for camera streams - used by both ESP32 devices and dashboard clients
pub async fn camera_websocket(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let device_id = path.into_inner();
    
    println!("[camera-ws] New WebSocket connection for device: {}", device_id);
    
    ws::start(
        CameraWebSocket { 
            device_id: device_id.clone() 
        },
        &req,
        stream,
    )
}

/// Function to broadcast camera frame to all connected dashboard clients
pub fn broadcast_camera_frame(device_id: &str, frame_data: Vec<u8>, frame_type: &str) {
    let streams = CAMERA_STREAMS.lock().unwrap();
    if let Some(device_streams) = streams.get(device_id) {
        let frame = CameraFrame {
            device_id: device_id.to_string(),
            frame_data,
            frame_type: frame_type.to_string(),
            timestamp: chrono::Utc::now(),
        };
        
        for addr in device_streams {
            addr.do_send(frame.clone());
        }
        
        println!("[camera-ws] Broadcasted frame to {} clients for device {}", 
            device_streams.len(), device_id);
    } else {
        println!("[camera-ws] No clients connected for device {}, dropping frame", device_id);
    }
}

/// Get the number of active streams for a device
pub fn get_active_streams_count(device_id: &str) -> usize {
    let streams = CAMERA_STREAMS.lock().unwrap();
    streams.get(device_id).map(|v| v.len()).unwrap_or(0)
}

/// Get all devices with active camera streams
pub fn get_active_camera_devices() -> Vec<String> {
    let streams = CAMERA_STREAMS.lock().unwrap();
    streams.keys().cloned().collect()
}