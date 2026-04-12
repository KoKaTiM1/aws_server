# worker-verify

SQS consumer that verifies detections using animal detection model.

## Responsibilities
- Consume `verify_requested` SQS messages (from worker-ingest)
- Fetch detection images from S3
- Run AI/ML verification (YOLO or similar)
- Update detection record with verification result (confidence score, animal type)
- Publish `verified_animals` message to notification queue
- Handle verification failures gracefully

## Environment Variables
- `QUEUE_URL_VERIFY` — verify_requested SQS queue URL
- `QUEUE_URL_NOTIFY` — verified_animals SQS queue URL (to publish)
- `DATABASE_URL` — RDS PostgreSQL connection string
- `S3_BUCKET` — S3 bucket for detection images
- `AWS_REGION` — AWS region
- `MODEL_PATH` — Path to YOLO model (or skip for placeholder)

## Verification States
- `pending` — awaiting verification
- `verified` — animal confirmed (confidence > threshold)
- `false_positive` — no animal detected
- `error` — verification failed

## Long Polling
- 20-second SQS wait time
- Batch processing (5 messages per poll - verification is CPU-intensive)

## Graceful Shutdown
- 30-second drain period before exit on SIGTERM
- Completes in-flight verifications where possible
