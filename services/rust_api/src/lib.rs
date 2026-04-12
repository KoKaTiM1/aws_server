pub mod auth;
pub mod config;
pub use crate::models::review_queue::{ReviewItem, ReviewStatus, UnclassifiedDetection};
pub use crate::services::minio_client::MinioClient;
#[cfg(test)]
// Re-export types
pub use crate::services::review_queue::ReviewQueueService;

// Declare modules
pub mod db;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;
pub mod utils;

// Re-export hardware types for convenience
pub use crate::models::hardware::{HardwarePayload, HardwareStatus, SensorData, SensorType};
