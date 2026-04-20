const express = require('express');
const cors = require('cors');
const helmet = require('helmet');
const rateLimit = require('express-rate-limit');
const { SecretsManagerClient, GetSecretValueCommand } = require('@aws-sdk/client-secrets-manager');
const pool = require('./db');

// API Key loaded from Secrets Manager at startup
let API_KEY = process.env.API_KEY || null;

// Firebase Admin SDK (initialized at startup)
let firebaseAdmin = null;

const app = express();
const PORT = process.env.PORT || 8080;

function extractApiKey(secretString) {
  if (!secretString) return null;
  const trimmed = secretString.trim();

  if (trimmed.startsWith('{')) {
    try {
      const parsed = JSON.parse(trimmed);
      return parsed.api_key || parsed.API_KEY || parsed.key || null;
    } catch (_) {
      return null;
    }
  }

  return trimmed;
}

// ============ MIDDLEWARE ============

// Security headers
app.use(helmet());

// CORS - Restrict to allowed origins (set via ALLOWED_ORIGINS env var)
app.use(cors({
  origin: process.env.ALLOWED_ORIGINS?.split(',') || [],
  credentials: true
}));

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

// Firebase token validation middleware (for mobile app endpoints)
// Extracts user UID from Firebase ID token in Authorization header
async function firebaseAuth(req, res, next) {
  // Only validate Firebase token for specific endpoints
  if (!req.path.startsWith('/api/notifications')) {
    return next();
  }

  // Skip if no Firebase admin initialized (fallback to query param for backward compatibility)
  if (!firebaseAdmin) {
    console.warn('⚠️ Firebase not initialized, allowing query param user_id (IDOR risk)');
    return next();
  }

  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'Missing Authorization header. Include Authorization: Bearer {idToken}' });
  }

  const idToken = authHeader.slice(7);
  try {
    const decodedToken = await firebaseAdmin.auth().verifyIdToken(idToken);
    // Attach verified UID to request for use in route
    req.user = { uid: decodedToken.uid, email: decodedToken.email };
    console.log(`✅ Firebase token verified for user: ${decodedToken.uid}`);
    next();
  } catch (error) {
    console.error('❌ Firebase token validation failed:', error.message);
    res.status(401).json({ error: 'Invalid or expired Firebase token' });
  }
}
app.use(firebaseAuth);

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
    res.status(500).json({ error: 'Failed to register user'});
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
    res.status(500).json({ error: 'Failed to update location'});
  }
});

/**
 * GET /api/notifications
 * Get notification history for a user (verified via Firebase token)
 * Query params: limit (default 50)
 * Header: Authorization: Bearer {idToken}
 */
app.get('/api/notifications', async (req, res) => {
  try {
    const { limit = 50 } = req.query;

    // If no Firebase auth, require user_id query param (for backward compatibility)
    // In production, require Firebase token always
    let user_id;
    if (req.user) {
      // User authenticated via Firebase token - use their UID
      // In production, map Firebase UID to database user_id
      // For now: extract user_id from request (with verification placeholder)
      user_id = req.query.user_id;
      if (!user_id) {
        return res.status(400).json({ error: 'user_id query parameter required' });
      }
      console.log(`✅ Verified Firebase user ${req.user.uid} requesting notifications for user_id: ${user_id}`);
      // TODO: Map Firebase UID (req.user.uid) to database user_id to prevent IDOR
    } else {
      // Fallback: for ESP devices or unverified requests
      user_id = req.query.user_id;
      if (!user_id) {
        return res.status(400).json({ error: 'Missing user_id query parameter' });
      }
      console.log(`⚠️ Unverified request for user_id: ${user_id}`);
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
    res.status(500).json({ error: 'Failed to fetch notifications' });
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
        let response;

        try {
          response = await smClient.send(new GetSecretValueCommand({ SecretId: 'eyedar-prod-api-keys-v3' }));
        } catch (_) {
          response = await smClient.send(new GetSecretValueCommand({ SecretId: 'eyedar-prod-api-keys' }));
        }

        API_KEY = extractApiKey(response.SecretString);

        if (!API_KEY) {
          throw new Error('API key secret was found but format is invalid. Expected plain text or JSON with api_key/API_KEY/key');
        }

        console.log('✅ API key loaded from Secrets Manager');
      } catch (smError) {
        console.error('❌ FATAL: Could not load API key from Secrets Manager:', smError.message);
        console.error('Set API_KEY environment variable or ensure secret exists: eyedar-prod-api-keys-v3');
        process.exit(1);  // Fail hard - let ECS restart the task
      }
    } else {
      const parsedApiKey = extractApiKey(API_KEY);
      if (!parsedApiKey) {
        console.error('❌ FATAL: API_KEY environment variable is invalid. Expected plain text or JSON with api_key/API_KEY/key');
        process.exit(1);
      }
      API_KEY = parsedApiKey;
      console.log('✅ API key loaded from environment variable');
    }

    // Initialize Firebase Admin SDK
    try {
      const admin = require('firebase-admin');
      let serviceAccountRaw = process.env.FIREBASE_CONFIG || process.env.FIREBASE_SERVICE_ACCOUNT;

      if (!serviceAccountRaw) {
        const smClient = new SecretsManagerClient({ region: process.env.AWS_REGION || 'us-east-1' });
        let response;

        try {
          response = await smClient.send(new GetSecretValueCommand({ SecretId: 'eyedar-prod-firebase-key-v3' }));
        } catch (_) {
          response = await smClient.send(new GetSecretValueCommand({ SecretId: 'eyedar-prod-firebase-key' }));
        }

        serviceAccountRaw = response.SecretString;
      }

      const serviceAccount = JSON.parse(serviceAccountRaw);

      admin.initializeApp({
        credential: admin.credential.cert(serviceAccount),
      });
      firebaseAdmin = admin;
      console.log('✅ Firebase Admin SDK initialized');
    } catch (fbError) {
      console.warn('⚠️ Firebase not initialized:', fbError.message);
      console.warn('Firebase token validation disabled - falling back to query param auth');
      // Don't exit - Firebase is optional for backward compatibility with hardware devices
    }

    // Test database connection
    await pool.query('SELECT NOW()');
    console.log('✅ Database connected');

    // Verify PostGIS is available (optional)
    try {
      const { rows } = await pool.query('SELECT PostGIS_Version()');
      console.log(`✅ PostGIS version: ${rows[0].postgis_version}`);
    } catch (pgErr) {
      console.warn('⚠️ PostGIS not available - geospatial queries will not work:', pgErr.message);
    }

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
