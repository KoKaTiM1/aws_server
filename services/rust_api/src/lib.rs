pub mod auth;
pub mod config;

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
