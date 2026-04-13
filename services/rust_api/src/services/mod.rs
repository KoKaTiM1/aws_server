pub mod geofence;
pub mod ws;
pub mod heartbeat;

// Circuit breaker for hardware failure detection
pub mod circuit_breaker;

// Business logic services
pub mod alert_service;
pub mod device_service;
pub mod image_service;
pub mod sqs_service;
