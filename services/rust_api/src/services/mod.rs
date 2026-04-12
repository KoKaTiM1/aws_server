pub mod geofence;
pub mod minio_client;
pub mod review_queue;
pub mod ws;
pub mod yolo_training;
pub mod heartbeat;
pub mod mqtt_bus;
pub mod mqtt_monitor;

// Circuit breaker for hardware failure detection
pub mod circuit_breaker;

// Business logic services
pub mod alert_service;
pub mod device_service;
pub mod image_service;
pub mod sqs_service;
