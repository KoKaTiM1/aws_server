-- DAR Database Schema
-- Migration 001: Initial schema with PostGIS support

-- Enable PostGIS extension for geospatial queries
CREATE EXTENSION IF NOT EXISTS postgis;

-- Sensors table (IoT devices)
CREATE TABLE IF NOT EXISTS sensors (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  sensor_id VARCHAR(100) UNIQUE NOT NULL,
  name VARCHAR(255),
  location GEOGRAPHY(POINT, 4326) NOT NULL,  -- PostGIS geography type
  status VARCHAR(50) DEFAULT 'active',  -- active, inactive, maintenance
  last_heartbeat TIMESTAMP,
  firmware_version VARCHAR(50),
  created_at TIMESTAMP DEFAULT NOW(),
  updated_at TIMESTAMP DEFAULT NOW()
);

-- Index for geospatial queries on sensors
CREATE INDEX idx_sensors_location ON sensors USING GIST(location);
CREATE INDEX idx_sensors_status ON sensors(status) WHERE status = 'active';

-- Detections table (animal detections from sensors)
CREATE TABLE IF NOT EXISTS detections (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  sensor_id UUID REFERENCES sensors(id),
  image_url VARCHAR(500),
  location GEOGRAPHY(POINT, 4326) NOT NULL,
  timestamp TIMESTAMP NOT NULL,
  verified BOOLEAN DEFAULT false,
  animal_type VARCHAR(100),  -- deer, wild_boar, fox, etc.
  confidence FLOAT,  -- AI confidence score
  processed_at TIMESTAMP,
  created_at TIMESTAMP DEFAULT NOW()
);

-- Indexes for detections
CREATE INDEX idx_detections_location ON detections USING GIST(location);
CREATE INDEX idx_detections_verified ON detections(verified) WHERE verified = true;
CREATE INDEX idx_detections_timestamp ON detections(timestamp DESC);
CREATE INDEX idx_detections_sensor ON detections(sensor_id);

-- Users table (app users/drivers)
CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  fcm_token VARCHAR(500) UNIQUE NOT NULL,  -- Firebase Cloud Messaging token (user identifier)
  device_platform VARCHAR(50),  -- android, ios, web
  app_version VARCHAR(50),
  current_location GEOGRAPHY(POINT, 4326),
  speed FLOAT,  -- km/h
  last_update TIMESTAMP NOT NULL,
  is_active BOOLEAN DEFAULT true,
  created_at TIMESTAMP DEFAULT NOW(),
  updated_at TIMESTAMP DEFAULT NOW()
);

-- Indexes for users
CREATE INDEX idx_users_location ON users USING GIST(current_location);
CREATE INDEX idx_users_last_update ON users(last_update DESC);
CREATE INDEX idx_users_active ON users(is_active) WHERE is_active = true;
CREATE INDEX idx_users_fcm_token ON users(fcm_token);

-- Alerts table (notifications sent to users)
CREATE TABLE IF NOT EXISTS alerts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  detection_id UUID REFERENCES detections(id),
  user_id UUID REFERENCES users(id),
  distance_km FLOAT NOT NULL,
  severity VARCHAR(20) NOT NULL,  -- danger, warning, info
  estimated_time_seconds INT,
  sent_at TIMESTAMP DEFAULT NOW(),
  fcm_status VARCHAR(50),  -- sent, delivered, failed, acknowledged
  fcm_response TEXT
);

-- Indexes for alerts
CREATE INDEX idx_alerts_detection ON alerts(detection_id);
CREATE INDEX idx_alerts_user ON alerts(user_id);
CREATE INDEX idx_alerts_sent_at ON alerts(sent_at DESC);
CREATE INDEX idx_alerts_severity ON alerts(severity);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
   NEW.updated_at = NOW();
   RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers for updated_at
CREATE TRIGGER update_sensors_updated_at BEFORE UPDATE ON sensors
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- View for active users (updated in last 5 minutes)
CREATE OR REPLACE VIEW active_users AS
SELECT *
FROM users
WHERE is_active = true
  AND last_update > NOW() - INTERVAL '5 minutes';

-- View for recent detections with sensor info
CREATE OR REPLACE VIEW recent_detections AS
SELECT 
  d.id,
  d.sensor_id,
  s.sensor_id as sensor_name,
  d.image_url,
  ST_Y(d.location::geometry) as latitude,
  ST_X(d.location::geometry) as longitude,
  d.timestamp,
  d.verified,
  d.animal_type,
  d.confidence,
  d.processed_at,
  d.created_at
FROM detections d
LEFT JOIN sensors s ON d.sensor_id = s.id
WHERE d.timestamp > NOW() - INTERVAL '24 hours'
ORDER BY d.timestamp DESC;

-- Sample function: Calculate distance between point and all active users
CREATE OR REPLACE FUNCTION find_nearby_users(
  detection_lat DOUBLE PRECISION,
  detection_lon DOUBLE PRECISION,
  max_distance_km FLOAT DEFAULT 5.0
)
RETURNS TABLE (
  user_id UUID,
  fcm_token VARCHAR,
  distance_km FLOAT,
  user_speed FLOAT,
  estimated_time_seconds INT
) AS $$
BEGIN
  RETURN QUERY
  SELECT 
    u.id,
    u.fcm_token,
    ST_Distance(
      u.current_location,
      ST_SetSRID(ST_MakePoint(detection_lon, detection_lat), 4326)::geography
    ) / 1000.0 AS distance_km,
    u.speed,
    CASE 
      WHEN u.speed IS NOT NULL AND u.speed > 5 THEN
        ROUND((ST_Distance(
          u.current_location,
          ST_SetSRID(ST_MakePoint(detection_lon, detection_lat), 4326)::geography
        ) / 1000.0) / u.speed * 3600)::INT
      ELSE
        NULL
    END AS estimated_time_seconds
  FROM active_users u
  WHERE ST_DWithin(
    u.current_location,
    ST_SetSRID(ST_MakePoint(detection_lon, detection_lat), 4326)::geography,
    max_distance_km * 1000
  )
  ORDER BY distance_km ASC;
END;
$$ LANGUAGE plpgsql;

-- Grant permissions (adjust as needed)
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO your_app_user;
-- GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO your_app_user;
-- GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO your_app_user;

COMMENT ON TABLE sensors IS 'IoT sensor devices deployed on roadsides';
COMMENT ON TABLE detections IS 'Animal detections captured by sensors';
COMMENT ON TABLE users IS 'Mobile app users (drivers)';
COMMENT ON TABLE alerts IS 'Notifications sent to users about nearby animals';
COMMENT ON FUNCTION find_nearby_users IS 'Find all active users within specified distance from a detection point';
