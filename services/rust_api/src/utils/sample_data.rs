use crate::routes::dashboard::{register_device, log_device_activity, log_alert};
use crate::models::dashboard::{DeviceHealth, DeviceActivity, ActivityType, AlertSummary, AlertSeverity};
use crate::models::hardware::{HardwareStatus, Location};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Populate sample dashboard data.
///
/// NOTE:
/// - Previously this function registered fake devices, activities, and alerts
///   so the dashboard had something to display before real telemetry arrived.
/// - Now that live ESP32/MQTT data is flowing into the dashboard, we disable
///   all fake seeding to ensure the UI reflects only real devices.
///
/// If you still want optional seeding for development, you can:
///   1. Add a configuration flag (e.g., env var) and only seed when enabled.
///   2. Or provide a separate `populate_sample_data_dev()` used only in dev builds.
pub fn populate_sample_data() {
    // No-op: we intentionally do not register any fake devices or alerts.
    // The dashboard will show only real data coming from the ESP32 devices.
    println!("Dashboard sample seeding disabled; using live ESP telemetry only.");
}
