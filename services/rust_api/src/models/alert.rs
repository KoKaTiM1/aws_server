use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertPayload {
    pub device_id: u32,
    pub message: String,
    pub timestamp: String,
    // Add more fields here as needed
}
