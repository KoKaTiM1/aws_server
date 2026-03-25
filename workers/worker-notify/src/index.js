/**
 * DAR Worker-Notify
 * 
 * Listens to verified_animals SQS queue and sends FCM notifications
 * to nearby drivers based on geospatial calculations.
 */

require('dotenv').config();
const { SQSClient, ReceiveMessageCommand, DeleteMessageCommand } = require('@aws-sdk/client-sqs');
const { SecretsManagerClient, GetSecretValueCommand } = require('@aws-sdk/client-secrets-manager');
const admin = require('firebase-admin');
const { Pool } = require('pg');

// AWS Configuration (SDK v3)
const sqs = new SQSClient({ region: process.env.AWS_REGION || 'us-east-1' });
const secretsManager = new SecretsManagerClient({ region: process.env.AWS_REGION || 'us-east-1' });

// Environment variables
const QUEUE_URL = process.env.SQS_QUEUE_URL_VERIFIED_ANIMALS;
const MAX_MESSAGES = 10;
const WAIT_TIME_SECONDS = 20; // Long polling
const MAX_ALERT_DISTANCE_KM = 1.0; // 1000 meters

// Database connection pool
let dbPool;

// Firebase Admin SDK
let firebaseInitialized = false;

/**
 * Initialize Firebase Admin SDK
 */
async function initializeFirebase() {
  if (firebaseInitialized) return;

  try {
    console.log('Initializing Firebase Admin SDK...');
    
    // Get Firebase service account from environment variable (loaded from Secrets Manager by ECS)
    const serviceAccount = JSON.parse(process.env.FIREBASE_SERVICE_ACCOUNT);

    admin.initializeApp({
      credential: admin.credential.cert(serviceAccount),
      databaseURL: `https://${serviceAccount.project_id}-default-rtdb.firebaseio.com`
    });

    firebaseInitialized = true;
    console.log('✅ Firebase Admin SDK initialized');
  } catch (error) {
    console.error('❌ Failed to initialize Firebase:', error);
    throw error;
  }
}

/**
 * Initialize PostgreSQL connection
 */
async function initializeDatabase() {
  try {
    console.log('Initializing database connection...');
    
    // Use credentials from environment variables (loaded from Secrets Manager by ECS)
    dbPool = new Pool({
      host: process.env.DB_HOST,
      port: parseInt(process.env.DB_PORT) || 5432,
      database: process.env.DB_NAME || 'eyedar',
      user: process.env.DB_USERNAME,
      password: process.env.DB_PASSWORD,
      max: 10,
      idleTimeoutMillis: 30000,
      connectionTimeoutMillis: 2000,
      ssl: {
        rejectUnauthorized: false,  // RDS uses self-signed cert
      },
    });

    // Test connection
    const client = await dbPool.connect();
    await client.query('SELECT NOW()');
    client.release();

    console.log('✅ Database connected');
  } catch (error) {
    console.error('❌ Database connection failed:', error);
    throw error;
  }
}

/**
 * Calculate severity based on distance
 */
function getSeverity(distanceKm) {
  if (distanceKm < 0.2) return 'danger';   // < 200m
  if (distanceKm < 0.5) return 'warning';  // 200m - 500m
  return 'info';                           // 500m - 1000m
}

/**
 * Get alert title based on severity
 */
function getAlertTitle(severity) {
  const titles = {
    danger: '🚨 DANGER! Animal on road',
    warning: '⚠️ Warning: Animal nearby',
    info: 'ℹ️ Animal detection'
  };
  return titles[severity] || 'Animal Alert';
}

/**
 * Find nearby users using PostGIS function
 */
async function findNearbyUsers(latitude, longitude, maxDistanceKm = MAX_ALERT_DISTANCE_KM) {
  const query = `
    SELECT * FROM find_nearby_users($1, $2, $3)
  `;
  
  try {
    const result = await dbPool.query(query, [latitude, longitude, maxDistanceKm]);
    return result.rows;
  } catch (error) {
    console.error('Error finding nearby users:', error);
    throw error;
  }
}

/**
 * Send FCM notification
 */
async function sendFCMNotification(user, detection) {
  const severity = getSeverity(user.distance_km);
  
  const message = {
    token: user.fcm_token,
    data: {
      id: detection.id,
      type: 'animal',
      latitude: detection.latitude.toString(),
      longitude: detection.longitude.toString(),
      distance_km: user.distance_km.toFixed(2),
      estimated_time_seconds: (user.estimated_time_seconds || 0).toString(),
      timestamp: detection.timestamp,
      animal_type: detection.animal_type || 'unknown',
      severity: severity
    },
    notification: {
      title: getAlertTitle(severity),
      body: `Animal detected ${user.distance_km.toFixed(1)}km ahead!`
    },
    android: {
      priority: 'high',
      notification: {
        channelId: 'animal_alerts',
        priority: 'high',
        sound: 'default'
      }
    },
    apns: {
      payload: {
        aps: {
          sound: 'default',
          badge: 1,
          alert: {
            title: getAlertTitle(severity),
            body: `Animal detected ${user.distance_km.toFixed(1)}km ahead!`
          }
        }
      }
    }
  };

  try {
    const response = await admin.messaging().send(message);
    console.log(`✅ Notification sent to user ${user.user_id}: ${response}`);
    return { success: true, messageId: response };
  } catch (error) {
    console.error(`❌ Error sending notification to user ${user.user_id}:`, error);
    return { success: false, error: error.message };
  }
}

/**
 * Save alert to database
 */
async function saveAlert(detectionId, userId, distanceKm, severity, estimatedTime, fcmStatus, fcmResponse) {
  const query = `
    INSERT INTO alerts (detection_id, user_id, distance_km, severity, estimated_time_seconds, fcm_status, fcm_response)
    VALUES ($1, $2, $3, $4, $5, $6, $7)
    RETURNING id
  `;
  
  try {
    const result = await dbPool.query(query, [
      detectionId,
      userId,
      distanceKm,
      severity,
      estimatedTime,
      fcmStatus,
      fcmResponse
    ]);
    return result.rows[0].id;
  } catch (error) {
    console.error('Error saving alert:', error);
    throw error;
  }
}

/**
 * Save detection to database (upsert)
 */
async function saveDetection(detection) {
  const query = `
    INSERT INTO detections (id, location, animal_type, confidence, image_url, timestamp, verified)
    VALUES ($1, ST_SetSRID(ST_MakePoint($2, $3), 4326)::geography, $4, $5, $6, $7, true)
    ON CONFLICT (id) DO NOTHING
    RETURNING id
  `;

  try {
    const result = await dbPool.query(query, [
      detection.id,
      detection.longitude,
      detection.latitude,
      detection.animal_type || 'unknown',
      detection.confidence || 0,
      detection.image_url || null,
      detection.timestamp || new Date().toISOString()
    ]);
    console.log(`✅ Detection ${detection.id} saved to database`);
    return result.rows[0]?.id || detection.id;
  } catch (error) {
    console.error('Error saving detection:', error);
    throw error;
  }
}

/**
 * Process verified animal detection
 */
async function processVerifiedAnimal(detection) {
  console.log(`Processing detection ${detection.id}...`);

  try {
    // Save detection to database first (required by foreign key constraint)
    await saveDetection(detection);

    // Find nearby users
    const nearbyUsers = await findNearbyUsers(
      detection.latitude,
      detection.longitude,
      MAX_ALERT_DISTANCE_KM
    );

    console.log(`Found ${nearbyUsers.length} nearby users`);

    if (nearbyUsers.length === 0) {
      console.log('No users nearby, skipping notifications');
      return { sent: 0, failed: 0 };
    }

    let sent = 0;
    let failed = 0;

    // Send notifications to all nearby users
    for (const user of nearbyUsers) {
      const result = await sendFCMNotification(user, detection);
      
      const severity = getSeverity(user.distance_km);
      const fcmStatus = result.success ? 'sent' : 'failed';
      const fcmResponse = result.success ? result.messageId : result.error;

      // Save alert to database
      await saveAlert(
        detection.id,
        user.user_id,
        user.distance_km,
        severity,
        user.estimated_time_seconds,
        fcmStatus,
        fcmResponse
      );

      if (result.success) {
        sent++;
      } else {
        failed++;
      }
    }

    console.log(`✅ Processed detection ${detection.id}: ${sent} sent, ${failed} failed`);
    return { sent, failed };

  } catch (error) {
    console.error(`Error processing detection ${detection.id}:`, error);
    throw error;
  }
}

/**
 * Poll SQS queue for messages
 */
async function pollQueue() {
  try {
    const params = {
      QueueUrl: QUEUE_URL,
      MaxNumberOfMessages: MAX_MESSAGES,
      WaitTimeSeconds: WAIT_TIME_SECONDS,
      VisibilityTimeout: 60, // 1 minute to process
      MessageAttributeNames: ['All']
    };

    const data = await sqs.send(new ReceiveMessageCommand(params));

    if (!data.Messages || data.Messages.length === 0) {
      return 0;
    }

    console.log(`Received ${data.Messages.length} messages`);

    for (const message of data.Messages) {
      try {
        const detection = JSON.parse(message.Body);

        // Process the detection
        await processVerifiedAnimal(detection);

        // Delete message from queue
        await sqs.send(new DeleteMessageCommand({
          QueueUrl: QUEUE_URL,
          ReceiptHandle: message.ReceiptHandle
        }));

        console.log(`✅ Message processed and deleted`);

      } catch (error) {
        console.error('Error processing message:', error);
        // Message will become visible again after VisibilityTimeout
      }
    }

    return data.Messages.length;

  } catch (error) {
    console.error('Error polling queue:', error);
    return 0;
  }
}

/**
 * Main worker loop
 */
async function main() {
  console.log('🚀 Starting DAR Worker-Notify...');
  console.log(`Environment: ${process.env.NODE_ENV || 'development'}`);
  console.log(`Queue URL: ${QUEUE_URL}`);

  try {
    // Initialize Firebase and Database
    await initializeFirebase();
    await initializeDatabase();

    console.log('✅ Worker initialized, starting to poll queue...\n');

    // Main loop
    while (true) {
      try {
        await pollQueue();
      } catch (error) {
        console.error('Error in main loop:', error);
        // Wait a bit before retrying
        await new Promise(resolve => setTimeout(resolve, 5000));
      }
    }

  } catch (error) {
    console.error('❌ Fatal error:', error);
    process.exit(1);
  }
}

// Handle graceful shutdown
process.on('SIGTERM', async () => {
  console.log('SIGTERM received, shutting down gracefully...');
  if (dbPool) {
    await dbPool.end();
  }
  process.exit(0);
});

process.on('SIGINT', async () => {
  console.log('SIGINT received, shutting down gracefully...');
  if (dbPool) {
    await dbPool.end();
  }
  process.exit(0);
});

// Start the worker
main();
