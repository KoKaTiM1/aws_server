# AWS Deployment Status and Next Steps

## Current State (2026-04-12)

### ✅ Completed
- **Terraform Infrastructure:** 121 resources defined across 6 module layers
- **Docker Images:** All 7 service Dockerfiles created
  - workers/api (Node.js)
  - workers/worker-ingest (Node.js)
  - workers/worker-verify (Node.js)
  - workers/worker-notify (Node.js)
  - services/rust_api (Rust multi-stage build)
  - services/mqtt-monitor (Node.js placeholder)
  - workers/dashboard (Node.js placeholder)
- **GitHub Actions Workflow:** Updated to build all 7 services
- **Database Schema:** PostgreSQL migrations with PostGIS support
- **CI/CD:** GitHub OIDC provider configured for AWS authentication

### ⚠️ Incomplete / In Progress
1. **Terraform Apply:** Partially deployed
   - ✅ VPC, subnets, security groups
   - ✅ ElastiCache Redis
   - ✅ S3 bucket (eyedar-prod-objects)
   - ✅ ECR repositories
   - ❌ Secrets Manager secret VALUES (containers created but no values)
   - ❌ RDS (waiting for DB secret value)
   - ❌ ECS Services (need RDS to complete setup)
   - ❌ ACM Certificate (placeholder domain, validation timeout)

2. **Docker Images:** Not built yet
   - Need GitHub Actions to build and push to ECR
   - OR local Docker build + manual ECR push

3. **ECS Services:** Not deployed yet
   - Waiting for complete Terraform apply
   - Need Docker images in ECR before services can start

## What's Needed to Complete Deployment

### Phase 1: Fix Terraform Issues (manual AWS Console actions)
1. **Add Secrets Manager Values**
   - Go to AWS Secrets Manager console
   - For secret `eyedar-prod-db-password`: Add actual RDS password
   - For secret `eyedar-prod-firebase-key`: Add Firebase service account JSON
   - For secret `eyedar-prod-api-keys`: Add API key for ESP device authentication

2. **Handle Pre-existing S3 Bucket**
   - Option A: Import existing bucket into Terraform state
     ```bash
     terraform import 'module.s3_objects.aws_s3_bucket.objects' eyedar-prod-objects
     ```
   - Option B: Update Terraform to use different bucket name

3. **Fix ACM Certificate** (if needed)
   - Option A: Use real domain name in terraform.tfvars
   - Option B: Disable ACM for initial testing (comment out in main.tf)

### Phase 2: Complete Terraform Deployment
```bash
cd infra/envs/prod
terraform apply -auto-approve
```

### Phase 3: Build and Push Docker Images
**Option A: Via GitHub Actions (Recommended)**
1. Push master branch to GitHub
2. GitHub Actions automatically:
   - Builds all 7 Docker images
   - Pushes to ECR
   - Updates ECS services
   - Deploys to cluster

**Option B: Manual Local Build**
```bash
# For each service:
docker build -t eyedar-api:latest workers/api
docker build -t eyedar-worker-ingest:latest workers/worker-ingest
# ... etc for all 7 services

# Login to ECR
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin 937115287175.dkr.ecr.us-east-1.amazonaws.com

# Tag and push
docker tag eyedar-api:latest 937115287175.dkr.ecr.us-east-1.amazonaws.com/eyedar-api:latest
docker push 937115287175.dkr.ecr.us-east-1.amazonaws.com/eyedar-api:latest
# ... repeat for all 7 services
```

### Phase 4: Verify Deployment
```bash
# Check ECS services
aws ecs list-services --cluster eyedar-prod --region us-east-1

# Check service status
aws ecs describe-services \
  --cluster eyedar-prod \
  --services eyedar-prod-api eyedar-prod-worker-ingest \
  --region us-east-1

# Check task status
aws ecs list-tasks --cluster eyedar-prod --region us-east-1
```

### Phase 5: Test End-to-End
1. Send detection via Rust API: `POST /api/v1/alerts`
2. Verify SQS messages processed
3. Check RDS for detection records
4. Verify worker-verify ran
5. Check worker-notify sent notification

## Environment Variables Required

Each service needs these environment variables (set in Secrets Manager):

**All Services:**
- `RDS_HOST`: Database endpoint
- `RDS_PORT`: 5432
- `RDS_DB_NAME`: postgres
- `RDS_PASSWORD`: (from Secrets Manager)

**Cache Services:**
- `REDIS_HOST`: ElastiCache endpoint
- `REDIS_PORT`: 6379

**Storage Services:**
- `S3_BUCKET_NAME`: eyedar-prod-objects
- `AWS_REGION`: us-east-1

**Queue Services:**
- `SQS_QUEUE_URL_DETECTION`: (created by Terraform)
- `SQS_QUEUE_URL_VERIFY`: (created by Terraform)
- `SQS_QUEUE_URL_NOTIFY`: (created by Terraform)

**Authentication:**
- `API_KEY_ESP`: (from Secrets Manager - for ESP device auth)
- `FIREBASE_SERVICE_ACCOUNT`: (from Secrets Manager - for mobile app auth)

## Architecture: From ESP Device to Mobile App

```
ESP32 Device
    ↓ (POST /api/v1/alerts with image)
Rust API (services/rust_api)
    ↓ (store image in S3, create detection record)
RDS PostgreSQL
    ↓ (publish detection_created event)
SQS Queue (detection_created)
    ↓
Worker-Ingest (workers/worker-ingest)
    ↓ (process images, extract data)
RDS PostgreSQL
    ↓ (publish verify_requested event)
SQS Queue (verify_requested)
    ↓
Worker-Verify (workers/worker-verify)
    ↓ (run ML/YOLO verification)
RDS PostgreSQL
    ↓ (publish verified_animals event)
SQS Queue (verified_animals)
    ↓
Worker-Notify (workers/worker-notify)
    ↓ (send FCM notification)
Firebase Cloud Messaging
    ↓
Mobile App (Flutter)
```

## File Checklist

### Dockerfiles ✅
- [x] workers/api/Dockerfile
- [x] workers/worker-ingest/Dockerfile
- [x] workers/worker-verify/Dockerfile
- [x] workers/worker-notify/Dockerfile
- [x] services/rust_api/Dockerfile
- [x] services/mqtt-monitor/Dockerfile
- [x] workers/dashboard/Dockerfile

### Code ✅
- [x] Rust API implementation
- [x] Worker-Ingest implementation
- [x] Worker-Verify implementation
- [x] Worker-Notify implementation
- [x] API implementation
- [⚠️] Dashboard (placeholder only)
- [⚠️] MQTT-Monitor (placeholder only)

### Terraform ✅
- [x] All modules defined
- [x] Variable defaults set
- [⏳] Apply in progress (needs Phase 1 fixes)

### CI/CD ✅
- [x] GitHub Actions workflow updated
- [x] ECR repositories created
- [x] Deploy policies configured
- [x] GitHub OIDC provider ready

## Known Issues & Workarounds

1. **ACM Certificate Validation Timeout**
   - Cause: Placeholder domain "api-placeholder.example.com" cannot be validated
   - Workaround: Use real domain or disable ACM for testing

2. **S3 Bucket Pre-exists**
   - Cause: Previous deployment attempt left bucket
   - Workaround: Import into Terraform state

3. **RDS Cannot Start**
   - Cause: DB password secret not populated in Secrets Manager
   - Workaround: Add password value manual in AWS Console

4. **ECS Services Not Starting**
   - Cause: Docker images not yet in ECR
   - Workaround: Build and push images first

## Next Immediate Actions

1. **Add Secrets Manager Values** (AWS Console, 5 minutes)
2. **Re-run `terraform apply`** (AWS CLI, 10 minutes)
3. **Push Docker changes to GitHub** (Git, 1 minute)
4. **Wait for GitHub Actions** to build images (5-10 minutes)
5. **Monitor ECS services** starting (5 minutes)
6. **Test with sample detection** (5 minutes)

## Total Time to Full Deployment: ~30-45 minutes
