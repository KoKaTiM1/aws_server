const { SQSClient, ReceiveMessageCommand, DeleteMessageCommand, SendMessageCommand } = require('@aws-sdk/client-sqs');
const { S3Client, GetObjectCommand } = require('@aws-sdk/client-s3');
const { Pool } = require('pg');
const { sdkStreamMixin } = require('@aws-sdk/util-stream-node');

const sqsClient = new SQSClient({ region: process.env.AWS_REGION || 'us-east-1' });
const s3Client = new S3Client({ region: process.env.AWS_REGION || 'us-east-1' });
const pool = new Pool({ connectionString: process.env.DATABASE_URL });

const QUEUE_URL_VERIFY = process.env.QUEUE_URL_VERIFY;
const QUEUE_URL_NOTIFY = process.env.QUEUE_URL_NOTIFY;
const S3_BUCKET = process.env.S3_BUCKET;

// ============ GRACEFUL SHUTDOWN ============
let isShuttingDown = false;

process.on('SIGTERM', async () => {
  console.log('[SIGTERM] Graceful shutdown started...');
  isShuttingDown = true;

  // Give in-flight verifications 30 seconds to complete
  setTimeout(async () => {
    await pool.end();
    process.exit(0);
  }, 30000);
});

// ============ VERIFICATION LOGIC ============
async function verifyDetection(detection, images) {
  // TODO: Implement actual AI/ML verification using YOLO or similar
  // For now, placeholder that analyzes metadata and returns confidence

  console.log(`[VERIFY] Running AI verification on ${images.length} image(s) for detection ${detection.detection_id}`);

  // Placeholder: simple heuristic based on image count and timestamp
  const imageCount = images.length;
  const hasImages = imageCount > 0;

  // Basic logic: if we have images, assume it's more likely to be real
  const placeholderConfidence = hasImages ? 0.85 : 0.40;
  const isVerified = placeholderConfidence > 0.7;

  return {
    verified: isVerified,
    confidence: placeholderConfidence,
    animal_type: 'wildlife', // TODO: Replace with actual YOLO output
    inference_time_ms: Math.random() * 500 + 100, // Placeholder timing
    error: null,
  };
}

// ============ DATABASE HELPERS ============
async function updateDetectionVerification(detectionId, verificationResult) {
  const client = await pool.connect();
  try {
    const sqlUpdate = `
      UPDATE detections
      SET verified = $1, confidence = $2, animal_type = $3, updated_at = NOW()
      WHERE id = $4;
    `;

    await client.query(sqlUpdate, [
      verificationResult.verified,
      verificationResult.confidence,
      verificationResult.animal_type,
      detectionId,
    ]);

    console.log(`[DB] ✅ Updated detection: ${detectionId}, verified=${verificationResult.verified}`);
  } finally {
    client.release();
  }
}

// ============ S3 IMAGE RETRIEVAL ============
async function fetchImageFromS3(imageUrl) {
  // TODO: Implement S3 image fetch and return as buffer
  // For now, placeholder that returns null (to be implemented later)
  console.log(`[S3] TODO: Fetch image from ${imageUrl}`);
  return null;
}

// ============ SQS HELPERS ============
async function publishNotifyMessage(verifyMessage, verificationResult) {
  const notifyMessage = {
    detection_id: verifyMessage.detection_id,
    device_id: verifyMessage.device_id,
    verified: verificationResult.verified,
    confidence: verificationResult.confidence,
    animal_type: verificationResult.animal_type,
    timestamp: verifyMessage.timestamp,
  };

  const params = {
    QueueUrl: QUEUE_URL_NOTIFY,
    MessageBody: JSON.stringify(notifyMessage),
    MessageGroupId: `device-${verifyMessage.device_id}`, // For FIFO queue (if applicable)
  };

  try {
    await sqsClient.send(new SendMessageCommand(params));
    console.log(`[SQS] ✅ Published verified_animals for detection: ${verifyMessage.detection_id}`);
  } catch (err) {
    console.error(`[SQS ERROR] Failed to publish notify message: ${err.message}`);
    throw err;
  }
}

async function deleteMessage(receiptHandle) {
  const params = {
    QueueUrl: QUEUE_URL_VERIFY,
    ReceiptHandle: receiptHandle,
  };

  try {
    await sqsClient.send(new DeleteMessageCommand(params));
    console.log(`[SQS] ✅ Deleted message from verify queue`);
  } catch (err) {
    console.error(`[SQS ERROR] Failed to delete message: ${err.message}`);
    throw err;
  }
}

// ============ MAIN CONSUMER LOOP ============
async function pollMessages() {
  try {
    const params = {
      QueueUrl: QUEUE_URL_VERIFY,
      MaxNumberOfMessages: 5, // CPU-intensive, process fewer at a time
      WaitTimeSeconds: 20,
    };

    const command = new ReceiveMessageCommand(params);
    const response = await sqsClient.send(command);

    if (!response.Messages) {
      return;
    }

    for (const message of response.Messages) {
      try {
        const verifyMessage = JSON.parse(message.Body);

        console.log(`[VERIFY] Processing detection: ${verifyMessage.detection_id}`);

        // 1. Fetch images from S3 (if needed)
        const images = [];
        for (const imageUrl of (verifyMessage.images || [])) {
          const imageBuffer = await fetchImageFromS3(imageUrl);
          if (imageBuffer) {
            images.push(imageBuffer);
          }
        }

        // 2. Run verification
        const verificationResult = await verifyDetection(verifyMessage, images);
        console.log(`[VERIFY] Result: verified=${verificationResult.verified}, confidence=${verificationResult.confidence}`);

        // 3. Update detection record in RDS
        await updateDetectionVerification(verifyMessage.detection_id, verificationResult);

        // 4. Publish verified_animals message
        await publishNotifyMessage(verifyMessage, verificationResult);

        // 5. Delete message from queue
        await deleteMessage(message.ReceiptHandle);

        console.log(`[VERIFY] ✅ Successfully verified detection: ${verifyMessage.detection_id}`);

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
  console.log('[START] worker-verify starting...');
  console.log(`[CONFIG] QUEUE_URL_VERIFY: ${QUEUE_URL_VERIFY}`);
  console.log(`[CONFIG] QUEUE_URL_NOTIFY: ${QUEUE_URL_NOTIFY}`);
  console.log(`[CONFIG] DATABASE_URL: ${process.env.DATABASE_URL ? 'configured' : 'MISSING'}`);
  console.log(`[CONFIG] S3_BUCKET: ${S3_BUCKET}`);

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
