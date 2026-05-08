# worker-verify

Verification worker for the Eyedar pipeline. It is responsible for taking work from the ingest step, fetching the stored image, and deciding whether the detection is a real animal or a false positive.

## What It Will Do

1. Read messages from `eyedar-prod-verify-requested`.
2. Fetch the image from the S3 bucket `eyedar-prod-objects-v2`.
3. Run YOLO-based inference inside the container.
4. Update the detection record with the verification result.
5. Publish confirmed detections to `eyedar-prod-verified-animals`.

## Runtime Shape

- The ECS task is intended to run Node.js for queue handling and Python for YOLO inference.
- The model execution is expected to happen inside the same container, so the image only needs one ECS task and one queue consumer.
- The worker is CPU-heavy compared to `worker-ingest`, so keep batch sizes smaller and route failures to DLQ.

## Environment Variables

- `QUEUE_URL_VERIFY` - SQS queue URL for `eyedar-prod-verify-requested`
- `QUEUE_URL_NOTIFY` - SQS queue URL for `eyedar-prod-verified-animals`
- `DATABASE_URL` - PostgreSQL connection string, or use `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USERNAME`, `DB_PASSWORD`
- `S3_BUCKET` - detection image bucket, usually `eyedar-prod-objects-v2`
- `AWS_REGION` - AWS region, usually `us-east-1`
- `MODEL_PATH` - path to the YOLO model weights if the container loads them from disk

## Verification States

- `pending` - waiting for verification
- `verified` - animal confirmed above threshold
- `false_positive` - no matching animal found
- `error` - inference or storage failure

## Build

The container is expected to include both Node.js and Python dependencies.

```powershell
cd workers/worker-verify
docker build -t eyedar-prod-worker-verify .
```

## Notes

- This worker is the place where YOLO integration belongs.
- It depends on S3 access, RDS write access, and SQS publish access.
- Confidence thresholds and class filtering should be kept in code, not in the README.
