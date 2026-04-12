use crate::models::{
    dashboard::{DeviceHealth, DeviceActivity, ActivityType},
    hardware::{HardwareStatus, Location},
};
use crate::routes::alerts::EspHardwareData;
use std::time::SystemTime;

/// Service responsible for translating ESP hardware payloads into
/// dashboard models (`DeviceHealth`, `DeviceActivity`, etc.).
pub struct DeviceService;

impl DeviceService {
    /// Build a `DeviceHealth` snapshot from ESP hardware data.
    ///
    /// This is used when the ESP sends a "hardware update" payload and we
    /// want to register/update the device in the dashboard registry.
    pub fn create_device_health_from_esp_data(
        device_id: u32,
        hardware_data: &EspHardwareData,
    ) -> DeviceHealth {
        // Derive high-level status and uptime percentage
        let status = Self::determine_device_status(hardware_data);
        let uptime_percentage = Self::calculate_uptime_percentage(hardware_data.uptime_seconds);

        // Optional location, if GPS fields are present
        let location = Self::extract_location(hardware_data);

        DeviceHealth {
            device_id,
            // For now, use a generic name based on the ID; this keeps us
            // independent from optional fields on `EspHardwareData`.
            device_name: Self::format_device_name(device_id, None),

            status,
            last_seen: SystemTime::now(),
            uptime_percentage,

            // We do not derive a percentage battery level here because the
            // mapping from voltage→percentage depends on your battery model.
            battery_level: None,

            // We do not have a reliable signal strength value in every build,
            // so default to `None`. You can wire this later if you add a
            // stable field to `EspHardwareData`.
            signal_strength: None,

            // Firmware version is unknown from the minimal fields we know
            // are present; use a sentinel string.
            firmware_version: "unknown".to_string(),

            location,

            // New console control fields – defaults for newly seen devices.
            mode: Some("production".to_string()),
            heartbeat_enabled: Some(true),
            mqtt_enabled: Some(true),
        }
    }

    /// Decide a coarse-grained device status from a subset of hardware metrics.
    ///
    /// Only uses fields that we know exist from the compiler error messages:
    ///  - `battery_voltage`
    ///  - `temperature_celsius`
    fn determine_device_status(hardware_data: &EspHardwareData) -> HardwareStatus {
        // Low-voltage protection
        if let Some(voltage) = hardware_data.battery_voltage {
            if voltage < 3.3 {
                return HardwareStatus::Error("Low battery voltage".to_string());
            }
        }

        // Over-temperature protection
        if let Some(temp) = hardware_data.temperature_celsius {
            if temp > 80.0 {
                return HardwareStatus::Error("High temperature".to_string());
            }
        }

        HardwareStatus::Online
    }

    /// Convert uptime seconds into an approximate uptime percentage over 24h.
    fn calculate_uptime_percentage(uptime_seconds: Option<u64>) -> f64 {
        let seconds = uptime_seconds.unwrap_or(0);
        // Simple 24h window model.
        let max = 24.0 * 3600.0;
        ((seconds as f64) / max * 100.0).clamp(0.0, 100.0)
    }

    /// Extract a `Location` from ESP hardware data, if latitude/longitude are present.
    ///
    /// Note: `Location.altitude` is `Option<f64>` while `EspHardwareData.altitude_meters`
    /// is `Option<f32>`, so we convert and upcast.
    fn extract_location(hardware_data: &EspHardwareData) -> Option<Location> {
        match (hardware_data.latitude, hardware_data.longitude) {
            (Some(lat), Some(lon)) => Some(Location {
                latitude: lat,
                longitude: lon,
                altitude: hardware_data.altitude_meters.map(|a| a as f64),
            }),
            _ => None,
        }
    }

    /// Create a dashboard activity entry from ESP metrics.
    ///
    /// Uses only fields that the compiler has confirmed exist:
    ///  - `cpu_usage_percent`
    ///  - `memory_free_kb`
    ///  - `temperature_celsius`
    ///  - `uptime_seconds`
    ///  - `battery_voltage`
    pub fn create_activity_from_esp_data(
        device_id: u32,
        hardware_data: &EspHardwareData,
    ) -> DeviceActivity {
        let details = format!(
            "CPU: {:?}%, free mem: {:?} KB, temp: {:?}°C, uptime: {:?}s, battery: {:?}V",
            hardware_data.cpu_usage_percent,
            hardware_data.memory_free_kb,
            hardware_data.temperature_celsius,
            hardware_data.uptime_seconds,
            hardware_data.battery_voltage,
        );

        DeviceActivity {
            device_id,
            activity_type: ActivityType::StatusUpdate,
            timestamp: SystemTime::now(),
            details,
            data_size: None,
        }
    }

    /// Adapter kept for existing call sites:
    ///
    /// `routes/alerts.rs` calls:
    /// `DeviceService::create_hardware_update_activity(device_id, hardware_data);`
    ///
    /// We implement it as a thin wrapper around `create_activity_from_esp_data`
    /// so you do not need to change the route code.
    pub fn create_hardware_update_activity(
        device_id: u32,
        hardware_data: &EspHardwareData,
    ) -> DeviceActivity {
        Self::create_activity_from_esp_data(device_id, hardware_data)
    }

    /// Format a human-friendly device name.
    ///
    /// Currently used only as a helper in `create_device_health_from_esp_data`.
    pub fn format_device_name(device_id: u32, chip_model: Option<&str>) -> String {
        match chip_model {
            Some(model) => format!("ESP32-{}-{:03}", model, device_id),
            None => format!("ESP32-Device-{:03}", device_id),
        }
    }

    /// Validate that a device ID is within the expected range.
    pub fn validate_device_id(device_id: u32) -> Result<(), String> {
        if device_id == 0 {
            return Err("Device ID cannot be zero".to_string());
        }
        if device_id > 9999 {
            return Err("Device ID too large (max: 9999)".to_string());
        }
        Ok(())
    }
}
