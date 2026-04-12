# worker-ingest

SQS consumer that ingests detection data into RDS database.

## Responsibilities
- Consume `detection_created` SQS messages (from rust_api POST /alerts)
- Parse detection data (device_id, images, metadata)
- Write detection record to RDS PostgreSQL
- Write image metadata to S3 references
- Publish `verify_requested` message to next queue for verification

## Environment Variables
- `QUEUE_URL_INGEST` — detection_created SQS queue URL
- `QUEUE_URL_VERIFY` — verify_requested SQS queue URL (to publish)
- `DATABASE_URL` — RDS PostgreSQL connection string
- `AWS_REGION` — AWS region

## Database Schema
- `detections` table — stores detection records
- `detection_images` table — links images to detections

## Long Polling
- 20-second SQS wait time for efficiency
- Batch processing (10 messages per poll)

## Graceful Shutdown
- 30-second drain period before exit on SIGTERM
- Completes in-flight messages before terminating
