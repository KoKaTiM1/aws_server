# worker-notify

Notification worker for the Eyedar pipeline. It consumes verified detections, looks up nearby users, writes alert records, and sends FCM push notifications.

## What It Does

1. Reads messages from `eyedar-prod-verified-animals`.
2. Saves the verified detection in PostgreSQL.
3. Finds nearby users with the PostGIS helper function.
4. Sends FCM notifications through Firebase Admin SDK.
5. Stores the notification outcome in the `alerts` table.

## Current Behavior

- Queue consumer uses 20-second long polling.
- Batch size is up to 10 messages per poll.
- Messages are processed concurrently with `Promise.allSettled`.
- The worker exits gracefully on `SIGTERM` or `SIGINT`.

## Environment Variables

- `SQS_QUEUE_URL_VERIFIED_ANIMALS` - SQS queue URL for verified detections
- `AWS_REGION` - AWS region, usually `us-east-1`
- `DB_HOST` - PostgreSQL host
- `DB_PORT` - PostgreSQL port
- `DB_NAME` - PostgreSQL database name
- `DB_USERNAME` - PostgreSQL username
- `DB_PASSWORD` - PostgreSQL password
- `FIREBASE_SERVICE_ACCOUNT` - Firebase service account JSON string

## Database Behavior

The worker expects these database objects to exist:

- `detections` - stores the verified detection record
- `alerts` - stores notification delivery results
- `find_nearby_users(latitude, longitude, max_distance_km)` - PostGIS function used to find recipients

## Build

```powershell
cd workers/worker-notify
docker build -t eyedar-prod-worker-notify .
```

## Local Run

```powershell
cd workers/worker-notify
npm install
$env:FIREBASE_SERVICE_ACCOUNT = Get-Content path\to\firebase-key.json -Raw
npm start
```

## Notes

- This worker currently handles the production notification path for verified animals.
- It depends on Firebase credentials and the database geospatial lookup path.
- When the new presence-based mobile flow is added, this README should be updated to match the new recipient resolution logic.