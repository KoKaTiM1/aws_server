// worker-ingest: SQS consumer for detection_created events
// Flow: SQS (detection_created) → Parse JSON → Write to RDS → Publish verify_requested

const { SQSClient, ReceiveMessageCommand, DeleteMessageCommand } = require('@aws-sdk/client-sqs');
const { Pool } = require('pg');

const sqsClient = new SQSClient({ region: process.env.AWS_REGION || 'us-east-1' });
const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
  max: 5,
  idleTimeoutMillis: 30000,
  connectionTimeoutMillis: 2000,
});

const QUEUE_URL_INGEST = process.env.QUEUE_URL_INGEST;
const QUEUE_URL_VERIFY = process.env.QUEUE_URL_VERIFY;
const POLL_WAIT_TIME = 20; // 20-second long polling

if (!QUEUE_URL_INGEST || !QUEUE_URL_VERIFY) {
  console.error('❌ QUEUE_URL_INGEST and QUEUE_URL_VERIFY environment variables required');
  process.exit(1);
}

/**
 * Write detection and images to RDS
 * @param {Object} detection - DetectionAlert parsed from JSON
 * @returns {Promise<number>} detection_id from database
 */
async function writeDetectionToDB(detection) {
  const client = await pool.connect();
  try {
    // Start transaction
    await client.query('BEGIN');

    // 1. Ensure device exists
    await client.query(
      `INSERT INTO devices (device_id, device_name)
       VALUES ($1, $2)
       ON CONFLICT (device_id) DO NOTHING`,
      [detection.device_id, `ESP32-Device-${detection.device_id}`]
    );

    // 2. Write detection record
    const detectionResult = await client.query(
      `INSERT INTO detections (device_id, message, severity, sensor_source, image_path, timestamp, verified)
       VALUES ($1, $2, $3, $4, $5, $6, $7)
       RETURNING id`,
      [
        detection.device_id,
        detection.message,
        detection.severity || 'low',
        detection.sensor_source || 'unknown',
        detection.image_path || null,
        new Date(detection.timestamp),
        false, // Not verified yet
      ]
    );

    const detectionId = detectionResult.rows[0].id;
    console.log(`✅ Detection ID ${detectionId} written to RDS`);

    // 3. Write image records (if provided)
    if (detection.images && detection.images.length > 0) {
      for (let i = 0; i < detection.images.length; i++) {
        await client.query(
          `INSERT INTO detection_images (detection_id, image_url, image_order)
           VALUES ($1, $2, $3)`,
          [detectionId, detection.images[i], i + 1]
        );
      }
      console.log(`📸 ${detection.images.length} images linked to detection ${detectionId}`);
    }

    // Commit transaction
    await client.query('COMMIT');
    return detectionId;
  } catch (error) {
    await client.query('ROLLBACK');
    throw error;
  } finally {
    client.release();
  }
}

/**
 * Publish verify_requested message to SQS
 * @param {number} detectionId - ID from database
 * @param {number} deviceId - ESP device ID
 */
async function publishVerifyRequest(detectionId, deviceId) {
  try {
    const messageBody = JSON.stringify({
      detection_id: detectionId,
      device_id: deviceId,
      timestamp: new Date().toISOString(),
      status: 'pending_verification',
    });

    // TODO: Send to SQS when queue is set up
    console.log(`📤 [TODO] Publish to QUEUE_URL_VERIFY: ${messageBody}`);
  } catch (error) {
    console.error('❌ Failed to publish verify request:', error);
  }
}

/**
 * Poll SQS for messages
 */
async function pollMessages() {
  try {
    const command = new ReceiveMessageCommand({
      QueueUrl: QUEUE_URL_INGEST,
      MaxNumberOfMessages: 10,
      WaitTimeSeconds: POLL_WAIT_TIME,
    });

    const response = await sqsClient.send(command);

    if (!response.Messages || response.Messages.length === 0) {
      console.log('⏳ No messages (long polling)');
      return;
    }

    console.log(`📥 Received ${response.Messages.length} message(s)`);

    for (const message of response.Messages) {
      try {
        // Parse SQS body
        const detection = JSON.parse(message.Body);
        console.log(`🎯 Processing detection from device ${detection.device_id}`);

        // Write to database
        const detectionId = await writeDetectionToDB(detection);

        // Publish to verify queue
        await publishVerifyRequest(detectionId, detection.device_id);

        // Delete from queue
        await sqsClient.send(
          new DeleteMessageCommand({
            QueueUrl: QUEUE_URL_INGEST,
            ReceiptHandle: message.ReceiptHandle,
          })
        );
        console.log(`✅ Message deleted from queue`);
      } catch (error) {
        console.error('❌ Failed to process message:', error);
        // TODO: Send to DLQ instead of deleting
      }
    }
  } catch (error) {
    console.error('❌ SQS poll error:', error);
  }
}

/**
 * Graceful shutdown
 */
let isShuttingDown = false;
async function shutdown(signal) {
  if (isShuttingDown) return;
  isShuttingDown = true;

  console.log(`\n🛑 ${signal} received, draining...`);
  await pool.end();
  console.log('✅ Shutdown complete');
  process.exit(0);
}

process.on('SIGTERM', () => shutdown('SIGTERM'));
process.on('SIGINT', () => shutdown('SIGINT'));

/**
 * Main loop
 */
async function main() {
  console.log('🚀 worker-ingest starting...');
  console.log(`📋 Queue: ${QUEUE_URL_INGEST}`);

  for (;;) {
    await pollMessages();
  }
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
