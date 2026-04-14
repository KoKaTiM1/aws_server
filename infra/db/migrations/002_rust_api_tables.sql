-- Rust API Dashboard Tables
-- Migration 002: Device health, alert history, and activity tracking

-- Device Health Monitoring Table
CREATE TABLE IF NOT EXISTS device_health (
    device_id INTEGER PRIMARY KEY,
    device_name VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,  -- Online, Offline, Maintenance, Error
    last_seen TIMESTAMP NOT NULL,
    uptime_percentage FLOAT,
    battery_level FLOAT,
    signal_strength INTEGER,
    firmware_version VARCHAR(100),
    location JSONB,  -- JSON location object
    mode VARCHAR(50),
    heartbeat_enabled BOOLEAN,
    mqtt_enabled BOOLEAN,
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Indexes for device health
CREATE INDEX IF NOT EXISTS idx_device_health_status ON device_health(status);
CREATE INDEX IF NOT EXISTS idx_device_health_last_seen ON device_health(last_seen DESC);

-- Alert History Table
CREATE TABLE IF NOT EXISTS alert_history (
    id SERIAL PRIMARY KEY,
    device_id INTEGER NOT NULL,
    device_name VARCHAR(255) NOT NULL,
    severity VARCHAR(50) NOT NULL,  -- Critical, High, Medium, Low
    message VARCHAR(500) NOT NULL,
    image_path VARCHAR(500),  -- S3 path to image
    acknowledged BOOLEAN DEFAULT false,
    acknowledged_at TIMESTAMP,
    timestamp TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Indexes for alert history
CREATE INDEX IF NOT EXISTS idx_alert_history_device ON alert_history(device_id);
CREATE INDEX IF NOT EXISTS idx_alert_history_timestamp ON alert_history(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_alert_history_severity ON alert_history(severity);
CREATE INDEX IF NOT EXISTS idx_alert_history_acknowledged ON alert_history(acknowledged) WHERE acknowledged = false;

-- Device Activities Table
CREATE TABLE IF NOT EXISTS device_activities (
    id SERIAL PRIMARY KEY,
    device_id INTEGER NOT NULL,
    activity_type VARCHAR(50) NOT NULL,  -- DataSent, AlertTriggered, CommandReceived, StatusChange
    timestamp TIMESTAMP NOT NULL,
    details TEXT,
    data_size BIGINT,  -- Size of data in bytes
    created_at TIMESTAMP DEFAULT NOW()
);

-- Indexes for device activities
CREATE INDEX IF NOT EXISTS idx_device_activities_device ON device_activities(device_id);
CREATE INDEX IF NOT EXISTS idx_device_activities_timestamp ON device_activities(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_device_activities_type ON device_activities(activity_type);

-- Trigger to update device_health.updated_at on updates
CREATE OR REPLACE FUNCTION update_device_health_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_device_health_trigger
BEFORE UPDATE ON device_health
FOR EACH ROW
EXECUTE FUNCTION update_device_health_updated_at();
