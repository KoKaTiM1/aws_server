const { SQSClient, ReceiveMessageCommand, DeleteMessageCommand, SendMessageCommand } = require('@aws-sdk/client-sqs');
const { Pool } = require('pg');
const { v4: uuidv4 } = require('uuid');

const sqsClient = new SQSClient({ region: process.env.AWS_REGION || 'us-east-1' });

// Construct connection string from environment variables or use individual config
const databaseUrl = process.env.DATABASE_URL;

const pool = new Pool(databaseUrl ?
  { connectionString: databaseUrl } :
  {
    host: process.env.DB_HOST,
    port: parseInt(process.env.DB_PORT) || 5432,
    database: process.env.DB_NAME,
    user: process.env.DB_USERNAME,
    password: process.env.DB_PASSWORD,
  }
);

const QUEUE_URL_INGEST = process.env.QUEUE_URL_INGEST;
const QUEUE_URL_VERIFY = process.env.QUEUE_URL_VERIFY;

// ============ GRACEFUL SHUTDOWN ============
let isShuttingDown = false;

process.on('SIGTERM', async () => {
  console.log('[SIGTERM] Graceful shutdown started...');
  isShuttingDown = true;

  // Give in-flight messages 30 seconds to complete
  setTimeout(async () => {
    await pool.end();
    process.exit(0);
  }, 30000);
});

// ============ DATABASE HELPERS ============
async function writeDetectionToRDS(detection) {
  const client = await pool.connect();
  try {
    const detectionId = uuidv4();

    // Insert detection record
    const sqlInsertDetection = `
      INSERT INTO detections
      (id, device_id, message, severity, sensor_source, timestamp, verified, created_at)
      VALUES ($1, $2, $3, $4, $5, $6, false, NOW())
      RETURNING id;
    `;

    const result = await client.query(sqlInsertDetection, [
      detectionId,
      detection.device_id,
      detection.message,
      detection.severity || 'medium',
      detection.sensor_source || 'camera',
      detection.timestamp,
    ]);

    console.log(`[DB] ✅ Inserted detection: ${detectionId}`);

    // Store image references if present
    if (detection.images && detection.images.length > 0) {
      const sqlInsertImages = `
        INSERT INTO detection_images (detection_id, image_url, image_index)
        VALUES ($1, $2, $3);
      `;

      for (let i = 0; i < detection.images.length; i++) {
        await client.query(sqlInsertImages, [
          detectionId,
          detection.images[i],
          i,
        ]);
      }
      console.log(`[DB] ✅ Stored ${detection.images.length} image references`);
    }

    return detectionId;
  } finally {
    client.release();
  }
}

// ============ SQS HELPERS ============
async function publishVerifyMessage(detectionId, detection) {
  const verifyMessage = {
    detection_id: detectionId,
    device_id: detection.device_id,
    images: detection.images || [],
    timestamp: detection.timestamp,
  };

  const params = {
    QueueUrl: QUEUE_URL_VERIFY,
    MessageBody: JSON.stringify(verifyMessage),
    MessageGroupId: `device-${detection.device_id}`, // For FIFO queue (if applicable)
  };

  try {
    await sqsClient.send(new SendMessageCommand(params));
    console.log(`[SQS] ✅ Published verify_requested for detection: ${detectionId}`);
  } catch (err) {
    console.error(`[SQS ERROR] Failed to publish verify message: ${err.message}`);
    throw err;
  }
}

async function deleteMessage(receiptHandle) {
  const params = {
    QueueUrl: QUEUE_URL_INGEST,
    ReceiptHandle: receiptHandle,
  };

  try {
    await sqsClient.send(new DeleteMessageCommand(params));
    console.log(`[SQS] ✅ Deleted message from queue`);
  } catch (err) {
    console.error(`[SQS ERROR] Failed to delete message: ${err.message}`);
    throw err;
  }
}

// ============ MAIN CONSUMER LOOP ============
async function pollMessages() {
  try {
    const params = {
      QueueUrl: QUEUE_URL_INGEST,
      MaxNumberOfMessages: 10,
      WaitTimeSeconds: 20,
    };

    const command = new ReceiveMessageCommand(params);
    const response = await sqsClient.send(command);

    if (!response.Messages) {
      return;
    }

    for (const message of response.Messages) {
      try {
        const detection = JSON.parse(message.Body);

        console.log(`[INGEST] Processing detection from device: ${detection.device_id}`);

        // 1. Write detection to RDS
        const detectionId = await writeDetectionToRDS(detection);

        // 2. Publish verify_requested message
        await publishVerifyMessage(detectionId, detection);

        // 3. Delete message from queue
        await deleteMessage(message.ReceiptHandle);

        console.log(`[INGEST] ✅ Successfully processed detection: ${detectionId}`);

      } catch (err) {
        console.error(`[ERROR] Failed to process message: ${err.message}`);
        // TODO: Send to DLQ if max retries exceeded
        // For now, leave in queue to retry (SQS visibility timeout will re-deliver)
      }
    }
  } catch (err) {
    console.error(`[POLL ERROR] ${err.message}`);
  }

  // Continue polling if not shutting down
  if (!isShuttingDown) {
    setTimeout(pollMessages, 1000);
  }
}

// ============ STARTUP ============
async function start() {
  console.log('[START] worker-ingest starting...');
  console.log(`[CONFIG] QUEUE_URL_INGEST: ${QUEUE_URL_INGEST}`);
  console.log(`[CONFIG] QUEUE_URL_VERIFY: ${QUEUE_URL_VERIFY}`);
  console.log(`[CONFIG] DATABASE_URL: ${process.env.DATABASE_URL ? 'configured' : 'MISSING'}`);

  // Verify database connection
  try {
    const client = await pool.connect();
    await client.query('SELECT NOW()');
    client.release();
    console.log('[DB] ✅ Connected to RDS');
  } catch (err) {
    console.error(`[DB ERROR] ${err.message}`);
    process.exit(1);
  }

  // Start polling
  console.log('[START] ✅ Worker ready. Polling for messages...');
  pollMessages();
}

start();
