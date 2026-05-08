# Project Review: AWS-SERVER (Eyedar System)
**Date: 2026-04-20 | Status: Tracking Open Issues + Next Phases**

---

## Active Focus: Next 3 Priorities

### **Priority 1: Complete Full API Test** (Phase 3.5)
**Status:** In Progress - Steps 1-2 verified, steps 3-4 in progress.

- [v] Verified image upload to S3
- [v] Verified S3 object listing
- [ ] Verify RDS detection record (run manual query)
- [ ] Verify SQS message in queue (run final check)

**Action:** Complete the RDS and SQS verification steps from test_api_entry_point.ps1

---

### **Priority 2: YOLO Integration** (Phase 4)
**Status:** Ready to start after API validation.

- [ ] Implement S3 image fetch in worker-verify
- [ ] Integrate YOLO model inference
- [ ] Add species + confidence fields to DB
- [ ] Implement verified/false_positive routing
- [ ] Test with real animal images

---

### **Priority 3: App Messages** (Phase 6)
**Status:** Ready after YOLO completion.

- [ ] Create device -> user mapping in DB
- [ ] Add user notification preferences  
- [ ] Update worker-notify to resolve recipients from DB
- [ ] Send rich FCM payloads (context + optional image)
- [ ] Validate delivery and retry strategy

---

## All Phases

### Phase 3: Load Testing (2-3 hours)
**Goal:** Test multi-device, multi-image concurrent uploads.

- [v] Run repeated 1k-message burst tests through the SQS pipeline
- [v] Send 3 devices x 5 images each = 15 images simultaneously
- [v] Measure upload latency (target: < 2s each)
- [v] Verify all images in S3
- [v] Verify all events persisted in DB
- [v] Confirm bursts drained to zero without queue buildup
- [ ] Verify no duplicates and no message loss
- [ ] Verify dashboard/monitoring reflects all alerts correctly
- [v] Observe CloudWatch throughput during bursts and capture per-minute delete rates

**Success:** Zero errors, full data integrity under concurrent load.

---

### Phase 3.5: Rust API Entry Point Validation (1-2 hours)
**Goal:** Verify the complete flow from API upload → S3 → RDS detection record → SQS message.

**Current state:**
- [v] Image upload to S3 is working.
- [v] S3 image listing confirmed.
- [ ] RDS detection record creation - in progress
- [ ] SQS message publication - in progress

**Verified steps:**
- [v] `POST /api/v1/alerts` endpoint receives request and returns detection ID
- [v] Image is stored in S3 (`eyedar-prod-objects` bucket)

**Still needed:**
- [ ] Verify detection record created in RDS (check `detections` table with detection ID)
- [ ] Verify `detection_created` message published to SQS queue
- [ ] Confirm ESP device ID, timestamp, and metadata are correctly persisted

**Tasks:**
- [v] Create test script to call `/api/v1/alerts` with image payload
- [v] Query S3 to confirm image storage
- [ ] Query RDS to confirm detection record (manual psql/pgAdmin query)
- [ ] Check SQS `eyedar-prod-detection-created` for message
- [ ] Verify message contains correct detection ID and metadata

**Success:** API call completes with image in S3, record in DB, and message in queue.

---

### Phase 4: Animal Detection (YOLO Integration) (4-6 hours)
**Goal:** Add automated animal verification to filter false positives.

**Current state:**
- `worker-verify` is a placeholder consumer with build/deploy already wired.
- Image fetch from S3 is not implemented yet.
- YOLO/model inference is not wired into the verification path.

**Architecture & Flow:**

Three separate ECS tasks work together:

```
1. Rust API (ecs_services/rust_api)
   ↓ Receives POST /api/v1/alerts with image
   ↓ Saves image to S3: eyedar-prod-objects/raw/<detection_id>.jpg
   ↓ Creates detection record in RDS
   ↓ Publishes message to SQS: eyedar-prod-detection-created
   
2. worker-ingest (ecs_services/worker-ingest)
   ↓ Consumes eyedar-prod-detection-created messages
   ↓ Processes/counts in RDS
   ↓ Publishes to SQS: eyedar-prod-verify-requested
   
3. worker-verify (ecs_services/worker-verify) ← YOLO runs here
   ↓ Consumes eyedar-prod-verify-requested messages
   ↓ Fetches image from S3
   ↓ Runs YOLO inference (Python subprocess)
   ↓ Routes based on confidence:
     - if confidence > 0.75: publish to eyedar-prod-verified-animals
     - else: mark as FALSE_POSITIVE in RDS
```

**Container Implementation:**

The `worker-verify` ECS container includes both Node.js and Python:

```dockerfile
FROM node:18 AS base

# Add Python runtime for YOLO
RUN apt-get update && apt-get install -y python3 python3-pip
RUN pip install ultralytics opencv-python

WORKDIR /app
COPY . .
RUN npm install

CMD ["node", "src/index.js"]
```

**Node.js → Python subprocess pattern:**

```javascript
// workers/worker-verify/src/index.js
const { spawn } = require('child_process');
const { S3Client, GetObjectCommand } = require('@aws-sdk/client-s3');

async function verifyDetection(detectionId, s3Key) {
  // 1. Fetch image from S3
  const imageBuffer = await s3.send(
    new GetObjectCommand({
      Bucket: 'eyedar-prod-objects',
      Key: s3Key
    })
  );
  
  // 2. Call Python YOLO inference
  const result = await runYoloInference(imageBuffer);
  
  // 3. Route based on confidence
  if (result.confidence > 0.75) {
    await sqs.send(new SendMessageCommand({
      QueueUrl: verifiedAnimalsUrl,
      MessageBody: JSON.stringify({
        detection_id: detectionId,
        species: result.species,
        confidence: result.confidence
      })
    }));
  } else {
    await updateDB({
      detection_id: detectionId,
      verification_status: 'FALSE_POSITIVE',
      confidence: result.confidence
    });
  }
}

function runYoloInference(imageBuffer) {
  return new Promise((resolve, reject) => {
    const python = spawn('python3', ['/app/yolo_inference.py']);
    
    python.stdin.write(imageBuffer);
    python.stdin.end();
    
    let output = '';
    python.stdout.on('data', data => output += data);
    python.on('close', (code) => {
      if (code === 0) {
        resolve(JSON.parse(output));
      } else {
        reject(new Error(`YOLO inference failed with code ${code}`));
      }
    });
  });
}
```

**Python YOLO inference script:**

```python
# workers/worker-verify/yolo_inference.py
from ultralytics import YOLO
import json
import sys

# Load YOLOv8 nano model at startup
model = YOLO('yolov8n.pt')

ANIMAL_CLASSES = [
  'dog', 'cat', 'bird', 'horse', 'cow', 'bear', 'deer', 
  'elephant', 'zebra', 'lion', 'tiger', 'fox', 'raccoon'
]

def infer(image_bytes):
    results = model.predict(source=image_bytes, conf=0.3)
    detections = []
    
    for r in results:
        for box in r.boxes:
            class_name = r.names[int(box.cls)]
            if class_name in ANIMAL_CLASSES:
                detections.append({
                    'species': class_name,
                    'confidence': float(box.conf),
                    'box': box.xyxy.tolist()
                })
    
    if detections:
        # Return highest confidence detection
        best = max(detections, key=lambda x: x['confidence'])
        return best
    return {'species': None, 'confidence': 0.0}

image_data = sys.stdin.buffer.read()
output = infer(image_data)
print(json.dumps(output))
```

**Build & Deploy Status:**
- ✅ `worker-verify` image build already in `.github/workflows/deploy.yml` (line 74-84)
- ✅ `worker-verify` deployment already in deploy step (line 123-127)
- Docker image pushed to ECR as `eyedar-prod-worker-verify:latest`
- ECS service deploys to cluster with force-new-deployment

**Still needed:**
- Create `yolo_inference.py` in `workers/worker-verify/`
- Update `workers/worker-verify/Dockerfile` to include Python + YOLOv8
- Implement S3 image fetch in `workers/worker-verify/src/index.js`
- Add species, confidence, verification_status fields to RDS detections table
- Define confidence thresholds and animal class list
- Add error handling and DLQ routing for bad images

**Tasks:**
- [ ] Update worker-verify Dockerfile with Python runtime
- [ ] Create yolo_inference.py with YOLOv8 model loading
- [ ] Implement S3 GetObject in worker-verify
- [ ] Wire subprocess call to Python inference
- [ ] Create DB migration for species + confidence + verification_status fields
- [ ] Implement confidence-based routing to verified_animals queue
- [ ] Test with real animal images from S3

**Success:** YOLO correctly classifies test images and routes to appropriate SQS queues based on confidence threshold.

---

### Phase 5: Dashboard Enhancements (3-4 hours)
**Goal:** Replace placeholder data with production-backed views.

**Tasks:**
- [ ] Replace CSV/test source with RDS queries
- [ ] Show S3 image thumbnails via signed URLs
- [ ] Add per-device tabs
- [ ] Add VERIFIED / PENDING / FALSE_POSITIVE sections
- [ ] Add full image modal

**Success:** Dashboard reflects live state from production data only.

---

### Phase 6: Mobile Notifications (2-3 hours)
**Goal:** Send user notifications when verified animal detections occur.

**Current state:**
- `worker-notify` already sends FCM messages and saves alert records.
- Recipient lookup currently depends on the existing `users` table and the `find_nearby_users` PostGIS function.
- The API currently registers and updates users by `fcm_token` and location, not by the proposed presence endpoint.

**Still needed:**
- Add the mobile presence endpoint (`POST /v1/device/presence`).
- Authenticate the request with the user token and derive identity server-side.
- Store last app state, server-side last seen time, location, and geohash/grouping data.
- Add device-to-user mapping and notification preference storage.
- Update `worker-notify` to resolve active recipients from the new presence data.
- Return richer FCM payloads with context, severity, and optional image metadata.
- Decide the exact mobile heartbeat and background update rules.

**Tasks:**
- [ ] Create device -> user mapping in DB
- [ ] Add user notification preferences
- [ ] Update worker-notify to resolve recipients from DB
- [ ] Send rich FCM payloads (context + optional image)
- [ ] Validate delivery and retry strategy

**Success:** Verified detections produce correct and timely mobile alerts.

---

### Phase 7: Throughput Scaling and Autoscaling (2-4 hours)
**Goal:** Raise sustained pipeline throughput beyond the current single-task baseline.

**Still needed:**
- Increase `worker_ingest` desired count in Terraform or via ECS service scaling.
- Add CloudWatch-based ECS autoscaling using SQS depth or age-of-oldest-message.
- Tune per-task batch size and worker parallelism if higher concurrency is required.
- Validate that RDS connection usage stays within safe limits under scale-out.

**Tasks:**
- [ ] Add ECS autoscaling for `worker-ingest`
- [ ] Test 2x and 3x task-count throughput against the same 1k burst load
- [ ] Tune worker batch sizes and connection pool settings if needed
- [ ] Confirm scale-in behavior after backlog clears

**Success:** Throughput scales predictably with additional worker tasks.

---

### Phase 8: Worker-Verify Feature Enablement (4-6 hours)
**Goal:** Replace the placeholder verification path with real image analysis.

**Still needed:**
- Implement S3 image fetch and preprocessing in `worker-verify`.
- Wire in YOLO/model inference and confidence-based routing.
- Persist species, confidence, and verification outcome in the database.
- Add failure handling, retry behavior, and DLQ routing for bad inputs.

**Tasks:**
- [ ] Add image download from S3
- [ ] Integrate YOLO inference runtime
- [ ] Add schema fields for verification metadata
- [ ] Test verified vs false-positive routing

**Success:** Verification decisions are automatic and reliable.

---

### Phase 9: Dashboard and Mobile Completion (5-7 hours)
**Goal:** Finish the user-facing surfaces with live production-backed data.

**Still needed:**
- Replace dashboard placeholder data with RDS-backed queries.
- Show thumbnails and full-image views from signed S3 URLs.
- Add verified/pending/false-positive states and per-device filtering.
- Add the mobile presence endpoint and notify active recipients from server-side presence data.

**Tasks:**
- [ ] Replace dashboard CSV/test source with RDS queries
- [ ] Add signed image URLs and modal view
- [ ] Implement device presence and notification preferences
- [ ] Validate end-to-end notification delivery

**Success:** The dashboard and mobile alerts reflect live production state.

---

### Phase 10: Documentation and README Maintenance (1-2 hours)
**Goal:** Keep the project and service READMEs aligned with the real infrastructure and runtime behavior.

**Still needed:**
- Keep the root README focused on build, bootstrap, and deploy steps.
- Keep `infra/README.md` aligned with the Terraform module tree and outputs.
- Keep worker READMEs aligned with queue names, env vars, and container behavior.
- Rebuild stale docs from the current system state instead of copying old notes forward.

**Tasks:**
- [ ] Refresh service READMEs when runtime behavior changes
- [ ] Refresh infra README when Terraform outputs or bootstrap steps change
- [ ] Remove or rewrite outdated operational notes when they drift from the stack

**Success:** The repository documentation stays current with the deployed system.

---

### Platform Setup Notes (from terraform_setup)
**Goal:** Record the infrastructure-level items that already exist in `infra/` so the review matches the repo structure.

**Already represented in the Terraform tree:**
- ECS cluster, task roles, and ECS service wiring.
- NAT, VPC endpoints, and centralized security groups.
- CloudWatch log groups, alarms/dashboards, and optional AWS Budgets.
- ACM/TLS, ALB routing, and WAF.
- SQS queue redrive/DLQ policy and IAM coverage.
- The optional `iam_humans` and `staging` pieces remain optional/out of scope unless we decide to enable them.

**What the review should still call out:**
- The infra folder already contains most of the platform foundation, so the remaining work is more about validation, deployment state, and feature completion than missing module structure.
- Any optional pieces that are intentionally disabled should stay labeled that way in the review.

**Success:** The review reflects the platform foundation already present in the repo rather than implying it still needs to be built.

---

### Deferred Service Activation

- `worker-verify` start is intentionally deferred until Phase 4 (YOLO integration).
- Current pipeline stability work is complete; remaining work is feature enablement.

---

## Issues Log

### Phase 1: Terraform Destroy/Apply
**Date:** 2026-04-14 to 2026-04-15

1. Dashboard references remained in Terraform after service consolidation.
- Root cause: Partial cleanup across modules.
- Resolution: Removed dashboard IAM/resources from compute role modules.
- Status: Fixed.

2. ECR destroy failures due to non-empty repositories.
- Root cause: Missing force delete behavior for image repositories.
- Resolution: Enabled force delete in ECR Terraform resources.
- Status: Fixed.

3. RDS destroy blocked by deletion protection.
- Root cause: Module default protection enabled for destroy cycle.
- Resolution: Set deletion protection false in prod module call.
- Status: Fixed.

4. S3 destroy blocked by non-empty bucket.
- Root cause: force_destroy not passed in prod module call.
- Resolution: Enabled force_destroy in prod module call.
- Status: Fixed.

5. Secrets recreate failure due to scheduled deletion window.
- Root cause: Secrets Manager recovery delay prevented same-name recreate.
- Resolution: Use forced deletion/manual cleanup path when needed.
- Status: Mitigated.

6. Dependency violations during destroy (security group/network timing).
- Root cause: Resource teardown ordering and attached ENIs.
- Resolution: Re-run destroy/cleanup after blocking dependencies removed.
- Status: Mitigated.

**Outcome:** Clean destroy + fresh apply succeeded; infrastructure reproducible.

---

### Phase 2: GitHub Actions CI/CD
**Date:** 2026-04-20

1. OIDC trust policy mismatch for repository subject.
- Root cause: Incorrect `sub` claim pattern in role trust policy.
- Resolution: Updated trust policy for `repo:KoKaTiM1/aws_server` refs.
- Status: Fixed.

2. Terraform secrets not fully wired from workflow to root module.
- Root cause: `TF_VAR_*` secrets existed in workflow but root variables/module wiring missing.
- Resolution: Added root vars and forwarded into secrets module.
- Status: Fixed.

3. DB secret shape incompatible with ECS key selectors.
- Root cause: ECS expected `username` and `password` JSON keys.
- Resolution: Enforced DB secret JSON structure used by ECS selectors.
- Status: Fixed.

4. ECR naming mismatch (`eyedar-*` built, `eyedar-prod-*` deployed).
- Root cause: Build workflow pushed non-prod repos while ECS task defs referenced prod repos.
- Resolution: Updated workflow to build/push `eyedar-prod-*` images.
- Status: Fixed.

5. Duplicate non-prod ECR repositories caused confusion.
- Root cause: Legacy repositories remained alongside prod repositories.
- Resolution: Deleted non-prod ECR repositories.
- Status: Fixed.

6. PostgreSQL authentication failures across services.
- Root cause: RDS master password diverged from Secrets Manager DB password.
- Resolution: RDS module now consumes the same `db_password` input used for secrets.
- Status: Fixed.

7. worker-verify not loaded in deployment.
- Root cause: Intentional deferment until YOLO phase.
- Resolution: Keep deferred and enable in Phase 4.
- Status: Deferred by design.

**Outcome:** End-to-end path stable: push -> build -> ECR -> ECS deploy.

---

### Phase 3: Load Testing and Worker Throughput
**Date:** 2026-05-05 to 2026-05-08

1. `worker-ingest` initially processed SQS messages serially and included a fixed repoll delay.
- Root cause: Per-message work was not parallelized inside the batch loop.
- Resolution: Reworked `worker-ingest` to process received SQS batches concurrently with `Promise.allSettled` and continuous long-polling.
- Status: Fixed.

2. Throughput verification required repeated burst testing after the concurrency fix.
- Root cause: The queue needed realistic load to prove the fix under burst conditions.
- Resolution: Ran repeated 1k-message SQS bursts, observed queue drain to zero, and captured CloudWatch delete-rate samples.
- Status: Verified.

3. CloudWatch sampling was initially unclear because immediate post-run windows returned no datapoints.
- Root cause: Metric aggregation timing did not line up with the first query window.
- Resolution: Queried `NumberOfMessagesDeleted` in the `us-east-1` region at 60s periods and captured the per-minute datapoints from the completed burst windows.
- Status: Verified.

**Outcome:** The worker ingest path is implemented and the throughput path is now measurable under load.

---

## Guardrails (Do Not Regress)

- Keep Terraform module layering pattern (foundation -> network -> data -> compute -> edge -> cicd)
- Keep PostGIS query paths and geospatial integrity checks
- Keep non-root Docker users and graceful SIGTERM handling
- Keep SQS + DLQ design pattern
- Keep KMS encryption and TLS requirements
- Keep Secrets Manager as source of runtime secrets
