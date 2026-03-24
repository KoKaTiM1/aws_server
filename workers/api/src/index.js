const express = require('express');
const cors = require('cors');
const helmet = require('helmet');
const rateLimit = require('express-rate-limit');
const { SecretsManagerClient, GetSecretValueCommand } = require('@aws-sdk/client-secrets-manager');
const pool = require('./db');

// API Key loaded from Secrets Manager at startup
let API_KEY = process.env.API_KEY || null;

const app = express();
const PORT = process.env.PORT || 8080;

// ============ MIDDLEWARE ============

// Security headers
app.use(helmet());

// CORS - Allow all origins for now (mobile app)
app.use(cors());

// Parse JSON
app.use(express.json({ limit: '1mb' }));

// Rate limiting (2000 requests per 5 minutes per IP - matches WAF)
const limiter = rateLimit({
  windowMs: 5 * 60 * 1000, // 5 minutes
  max: 2000,
  message: { error: 'Too many requests, please try again later' },
  standardHeaders: true,
  legacyHeaders: false,
});
app.use(limiter);

// Request logging
app.use((req, res, next) => {
  const start = Date.now();
  res.on('finish', () => {
    const duration = Date.now() - start;
    console.log(`${req.method} ${req.path} ${res.statusCode} ${duration}ms`);
  });
  next();
});

// API Key authentication middleware
// Skips /health and / (root) endpoints
function apiKeyAuth(req, res, next) {
  // Skip auth for health check and root
  if (req.path === '/health' || req.path === '/') {
    return next();
  }

  const providedKey = req.headers['x-api-key'];
  if (!providedKey) {
    return res.status(401).json({ error: 'Missing API key. Include X-API-Key header.' });
  }

  if (!API_KEY) {
    console.error('❌ API_KEY not loaded - rejecting request');
    return res.status(503).json({ error: 'Service not ready' });
  }

  // Constant-time comparison to prevent timing attacks
  if (providedKey.length !== API_KEY.length) {
    return res.status(403).json({ error: 'Invalid API key' });
  }
  
  const a = Buffer.from(providedKey);
  const b = Buffer.from(API_KEY);
  const crypto = require('crypto');
  if (!crypto.timingSafeEqual(a, b)) {
    return res.status(403).json({ error: 'Invalid API key' });
  }

  next();
}
app.use(apiKeyAuth);

// ============ ROUTES ============

// Health check endpoint (used by ALB target group)
app.get('/health', (req, res) => {
  res.status(200).json({ status: 'healthy', service: 'eyedar-api', timestamp: new Date().toISOString() });
});

// Root endpoint
app.get('/', (req, res) => {
  res.json({
    service: 'DAR API Service',
    version: '1.0.0',
    endpoints: [
      'POST /api/users - Register user',
      'POST /api/v1/location - Update user location',
      'GET /api/notifications - Get notification history',
    ],
  });
});

// ============ API ENDPOINTS ============

/**
 * POST /api/users
 * Register a new user or update existing user by FCM token
 * Body: { fcm_token, device_platform, app_version }
 */
app.post('/api/users', async (req, res) => {
  try {
    const { fcm_token, device_platform, app_version } = req.body;

    // Validation
    if (!fcm_token || typeof fcm_token !== 'string' || fcm_token.length < 10) {
      return res.status(400).json({ error: 'Invalid fcm_token' });
    }

    // Insert or update user (upsert on fcm_token conflict)
    const result = await pool.query(
      `INSERT INTO users (fcm_token, device_platform, app_version, last_update, is_active, created_at, updated_at)
       VALUES ($1, $2, $3, NOW(), true, NOW(), NOW())
       ON CONFLICT (fcm_token) 
       DO UPDATE SET 
         device_platform = EXCLUDED.device_platform,
         app_version = EXCLUDED.app_version,
         updated_at = NOW(),
         is_active = true
       RETURNING id, fcm_token, created_at`,
      [fcm_token, device_platform || 'unknown', app_version || '1.0.0']
    );

    const user = result.rows[0];
    console.log(`✅ User registered: ${user.id} (${device_platform})`);

    res.status(201).json({
      id: user.id,
      fcm_token: user.fcm_token,
      created_at: user.created_at,
    });
  } catch (error) {
    console.error('❌ Error registering user:', error);
    res.status(500).json({ error: 'Failed to register user', details: error.message });
  }
});

/**
 * POST /api/v1/location
 * Update user location (MUST be called at least every 5 minutes to stay active)
 * Body: { fcm_token, latitude, longitude, speed (km/h), timestamp }
 */
app.post('/api/v1/location', async (req, res) => {
  try {
    const { fcm_token, latitude, longitude, speed, timestamp } = req.body;

    // Validation
    if (!fcm_token) {
      return res.status(400).json({ error: 'Missing fcm_token' });
    }
    if (typeof latitude !== 'number' || latitude < -90 || latitude > 90) {
      return res.status(400).json({ error: 'Invalid latitude' });
    }
    if (typeof longitude !== 'number' || longitude < -180 || longitude > 180) {
      return res.status(400).json({ error: 'Invalid longitude' });
    }

    // Update user location using PostGIS ST_SetSRID and ST_MakePoint
    // GEOGRAPHY uses lon, lat order (not lat, lon!)
    const result = await pool.query(
      `UPDATE users 
       SET current_location = ST_SetSRID(ST_MakePoint($2, $3), 4326)::geography,
           speed = $4,
           last_update = NOW(),
           is_active = true,
           updated_at = NOW()
       WHERE fcm_token = $1
       RETURNING id, is_active`,
      [fcm_token, longitude, latitude, speed || 0]
    );

    if (result.rows.length === 0) {
      // User not found - they need to register first
      return res.status(404).json({ 
        error: 'User not found. Please register first at POST /api/users',
        fcm_token: fcm_token,
      });
    }

    const user = result.rows[0];
    res.status(200).json({
      status: 'ok',
      is_active: user.is_active,
      user_id: user.id,
    });
  } catch (error) {
    console.error('❌ Error updating location:', error);
    res.status(500).json({ error: 'Failed to update location', details: error.message });
  }
});

/**
 * GET /api/notifications
 * Get notification history for a user
 * Query params: user_id (UUID), limit (default 50)
 */
app.get('/api/notifications', async (req, res) => {
  try {
    const { user_id, limit = 50 } = req.query;

    if (!user_id) {
      return res.status(400).json({ error: 'Missing user_id query parameter' });
    }

    // Fetch alerts with detection details
    const result = await pool.query(
      `SELECT 
         a.id,
         a.detection_id,
         a.distance_km,
         a.severity,
         a.estimated_time_seconds,
         a.sent_at,
         d.animal_type,
         ST_Y(d.location::geometry) as latitude,
         ST_X(d.location::geometry) as longitude,
         d.timestamp as detection_timestamp
       FROM alerts a
       JOIN detections d ON a.detection_id = d.id
       WHERE a.user_id = $1
       ORDER BY a.sent_at DESC
       LIMIT $2`,
      [user_id, Math.min(parseInt(limit) || 50, 200)]
    );

    res.status(200).json({
      alerts: result.rows,
      count: result.rows.length,
    });
  } catch (error) {
    console.error('❌ Error fetching notifications:', error);
    res.status(500).json({ error: 'Failed to fetch notifications', details: error.message });
  }
});

// ============ ERROR HANDLING ============

// 404 handler
app.use((req, res) => {
  res.status(404).json({ error: 'Endpoint not found' });
});

// Global error handler
app.use((err, req, res, next) => {
  console.error('❌ Unhandled error:', err);
  res.status(500).json({ error: 'Internal server error' });
});

// ============ STARTUP ============

async function startup() {
  try {
    // Load API key from Secrets Manager
    if (!API_KEY) {
      try {
        const smClient = new SecretsManagerClient({ region: process.env.AWS_REGION || 'us-east-1' });
        const response = await smClient.send(new GetSecretValueCommand({ SecretId: 'eyedar-prod/api-key' }));
        API_KEY = response.SecretString;
        console.log('✅ API key loaded from Secrets Manager');
      } catch (smError) {
        console.error('⚠️ Could not load API key from Secrets Manager:', smError.message);
        console.error('⚠️ API endpoints will reject requests until API_KEY env var is set');
      }
    } else {
      console.log('✅ API key loaded from environment variable');
    }

    // Test database connection
    await pool.query('SELECT NOW()');
    console.log('✅ Database connected');

    // Verify PostGIS is available
    const { rows } = await pool.query('SELECT PostGIS_Version()');
    console.log(`✅ PostGIS version: ${rows[0].postgis_version}`);

    // Start server
    app.listen(PORT, '0.0.0.0', () => {
      console.log(`🚀 DAR API Service listening on port ${PORT}`);
      console.log(`Environment: ${process.env.NODE_ENV || 'production'}`);
      console.log(`Database: ${process.env.DB_HOST}`);
    });
  } catch (error) {
    console.error('❌ Startup failed:', error);
    process.exit(1);
  }
}

// Graceful shutdown
process.on('SIGTERM', async () => {
  console.log('SIGTERM received, shutting down gracefully...');
  await pool.end();
  process.exit(0);
});

startup();
