# worker-ingest

Ingest worker for the Eyedar pipeline. It consumes new detections from SQS, writes them to PostgreSQL, and forwards verified work to the next queue.

## What It Does

1. Reads messages from `eyedar-prod-detection-created`.
2. Parses the detection payload from the Rust API.
3. Inserts the detection into PostgreSQL.
4. Publishes a `verify_requested` message to the next queue.
5. Deletes the source message after the write succeeds.

## Current Behavior

- Batch size: up to 10 messages per receive call.
- Processing: concurrent within each batch using `Promise.allSettled`.
- Polling: 20-second long polling with no fixed repoll delay between empty batches.
- Shutdown: waits for in-flight work to finish before exit.

## Environment Variables

- `QUEUE_URL_INGEST` - SQS queue URL for `eyedar-prod-detection-created`
- `QUEUE_URL_VERIFY` - SQS queue URL for `eyedar-prod-verify-requested`
- `DATABASE_URL` - PostgreSQL connection string, or use `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USERNAME`, `DB_PASSWORD`
- `AWS_REGION` - AWS region, usually `us-east-1`

## Database Tables

- `detections` - stores the main detection record
- `detection_images` - stores image references attached to a detection

## Local Run

```powershell
cd workers/worker-ingest
npm install
npm start
```

## Build

```powershell
docker build -t eyedar-prod-worker-ingest .
```

## Notes

- The worker expects the upstream API to publish detection payloads consistently.
- SQS redrive/DLQ handling is configured in Terraform.
- This worker is the scaling point for throughput improvements.
