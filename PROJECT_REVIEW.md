# Project Review: AWS-SERVER (Eyedar System)
**Date: 2026-04-14 | Status: Core Infrastructure Running ✅**

---

## Current Status

### ✅ What's Working
- **Infrastructure:** VPC, RDS, ElastiCache, S3, ECR, ECS, ALB, KMS, DNS
- **Services:** Rust API, Worker-Notify, Worker-Ingest (all running)
- **Image Pipeline:** ESP32 → API → S3 upload → SQS queue → Worker processing ✅
- **Dashboard:** Consolidated into Rust API, now single service
- **Security:** CSP headers fixed, SSL/TLS enabled, Firebase credentials updated
- **Database:** RDS connected + working (SSL required)

### 🔄 What's In Progress
- Dashboard showing test CSV data (ready to swap for RDS queries)
- YOLO model not yet integrated

---

## Next Workflow (Phases to Complete Before Publishing)

### Phase 1: Infrastructure Validation (1-2 hours)
**Goal:** Ensure Terraform is reproducible and identify any missing configs

- [ ] **Terraform destroy** - Backup data first, then destroy current stack
- [ ] **Terraform apply** - Fresh deploy from clean state
- [ ] **Validation checklist:**
  - [ ] All resources created (VPC, RDS, S3, ECR, ECS, ALB, IAM roles)
  - [ ] RDS accessible with correct credentials
  - [ ] S3 bucket working
  - [ ] Security groups allow traffic correctly
  - [ ] GitHub OIDC role has necessary permissions
  
**Deliverable:** Document any missing ARNs/policies that need to be added to Terraform

---

### Phase 2: GitHub Actions CI/CD Testing (1-2 hours)
**Goal:** Verify automated build → push → deploy works end-to-end

- [ ] Trigger workflow with small code change
- [ ] Verify: Build succeeds → Image pushed to ECR → ECS service updates
- [ ] No manual steps required
- [ ] All 6 services build successfully

**Success:** One commit to `main` deploys everything automatically

---

### Phase 3: Load Testing (2-3 hours)
**Goal:** Test multi-device, multi-image concurrent uploads

- [ ] Send 3 devices × 5 images each = 15 images simultaneously
- [ ] Measure upload latency (target: < 2s each)
- [ ] Verify: All images in S3 ✓, All in database ✓, No duplicates ✓
- [ ] Dashboard shows all alerts correctly

**Success:** Zero errors, all data persisted correctly

---

### Phase 4: Animal Detection (YOLO Integration) (4-6 hours)
**Goal:** Add automated animal verification to filter false positives

**Architecture:**
```
Image → S3 raw/ folder
      → worker-verify calls YOLO
      → If animal detected: move to verified/ + send notification
      → If no animal: move to false_positive/
```

**Tasks:**
- [ ] Organize S3 with folder structure: `raw/`, `to_verify/`, `verified/`, `false_positive/`
- [ ] Integrate YOLO model in worker-verify
- [ ] Update database to store detection results (species, confidence)
- [ ] Test with real images

**Success:** YOLO correctly identifies/rejects test images

---

### Phase 5: Dashboard UI Enhancements (3-4 hours)
**Goal:** Display real detection images and per-device tabs

**Changes:**
- [ ] Query RDS instead of CSV test data
- [ ] Display S3 image thumbnails with signed URLs
- [ ] Add per-device tabs showing all detections
- [ ] Separate sections: VERIFIED / PENDING / FALSE_POSITIVE
- [ ] Full image viewer modal when clicking thumbnail

**Success:** Dashboard shows all uploaded images organized by device

---

### Phase 6: Mobile Notifications (2-3 hours)
**Goal:** Send app notifications when animals are detected

**Flow:**
```
YOLO verifies animal
  → worker-notify fetches device owner + preferences
  → Sends FCM notification to user app
  → User sees alert: "Animal detected near Camera 1"
  → Taps to view detection details
```

**Tasks:**
- [ ] Create device → user mapping in database
- [ ] Add user notification preferences table
- [ ] Enhance worker-notify to fetch user tokens
- [ ] Send rich notifications with image thumbnail

**Success:** Push notification received on phone with animal detection

---

## Important: Keep These Unchanged
- Terraform module structure (foundation → network → data → compute → edge → cicd)
- PostGIS geospatial queries
- Non-root users in Docker
- SQS + Dead Letter Queue pattern
- S3 VPC endpoint
- KMS encryption
- SIGTERM graceful shutdown in all services
- Security headers (CSP, HSTS)
- SSL/TLS for database

---

---

## Issues Found During Phases (Tracking Log)

### Phase 1: Terraform Destroy/Apply
**Date Started:** 2026-04-14

#### Issues Encountered:

1. **Dashboard service references remained in Terraform after code cleanup**
   - **Files affected:** 
     - `infra/modules/40-compute/ecs_task_roles/main.tf` (lines 323-370)
     - `infra/modules/40-compute/ecs_task_roles/outputs.tf` (lines 26-29)
   - **Root cause:** When dashboard was consolidated into Rust API, Terraform module files weren't updated in sync
   - **Resolution:** Removed dashboard IAM role, dashboard policy, and dashboard output from above files
   - **Status:** ✅ FIXED - Terraform validate now passes
   - **Lesson Learned:** When removing services, must update ALL layers: code, Docker, CI/CD, AND Terraform modules

2. **Terraform state contains dashboard resources**
   - **Finding:** `terraform plan -destroy` shows dashboard ECR repo, security group, ALB target group, ECS service, task definition, and log group still in state
   - **Impact:** Destroying infrastructure will remove these resources (which is desired)
   - **Note:** This is expected - the state was synced with active resources before code changes
   - **Next step:** When running `terraform destroy`, these will be cleaned up as expected

3. **ECR repositories have force_delete restrictions**
   - **Finding:** `terraform destroy` fails with "ECR Repository not empty" errors
   - **Root cause:** AWS ECR repos contain Docker images; Terraform needs `force_delete = true` to allow deletion
   - **Affected repositories:** api, rust_api, worker_ingest, worker_verify, worker_notify, mqtt_monitor, dashboard
   - **Resolution:** Added `force_delete = true` to all aws_ecr_repository resources in infra/modules/40-compute/ecr/main.tf
   - **Status:** ✅ FIXED in code (applied in next terraform apply)

4. **RDS has deletion protection enabled by default**
   - **Finding:** `terraform destroy` fails with "Cannot delete protected DB Instance" error
   - **Root cause:** RDS in rds_postgres module defaults to `deletion_protection = true`
   - **Resolution:** Added `deletion_protection = false` to RDS module call in infra/envs/prod/main.tf
   - **Status:** ✅ FIXED in code (applied in next terraform apply)

5. **S3 bucket force_destroy not explicitly enabled**
   - **Finding:** `terraform destroy` fails with "bucket you tried to delete is not empty" error
   - **Root cause:** S3 module has `force_destroy = var.force_destroy` but prod environment doesn't pass the variable
   - **Resolution:** Added `force_destroy = true` to s3_objects module call in infra/envs/prod/main.tf
   - **Status:** ✅ FIXED in code (applied in next terraform apply)

6. **Secrets Manager deletion scheduling conflict**
   - **Finding:** After destroy, terraform apply fails with "secret...already scheduled for deletion" for db-password, firebase-key, and api-keys secrets
   - **Root cause:** AWS Secrets Manager schedules secrets for deletion over 7 days; can't recreate with same name until deletion completes
   - **Impact:** Terraform can't reapply while secrets are in deletion state
   - **Workaround:** Either (a) wait for 7-day schedule, (b) manually cancel deletion in AWS Console, or (c) use sleep/retry in CI/CD
   - **Status:** ⚠️ REQUIRES MANUAL INTERVENTION or wait for deletion schedule
   - **Solution for next apply:** Skip deletion scheduler or force immediate deletion via AWS API

7. **Dashboard ECR repository still not force_deleted**
   - **Finding:** Even with force_delete = true in code, dashboard ECR fails to delete
   - **Root cause:** force_delete setting may not have been applied to existing state
   - **Impact:** Dashboard ECR repo remains, blocking full destroy
   - **Resolution:** Manually delete via AWS CLI: `aws ecr delete-repository --repository-name eyedar-prod-dashboard --force`
   - **Status:** ⚠️ REQUIRES MANUAL AWS CLI COMMAND

8. **Security group has dependent resources**
   - **Finding:** ECS dashboard security group (sg-06bed288adb81eb94) has dependent objects preventing deletion
   - **Root cause:** Network interfaces or other resources still attached; race condition in destroy order
   - **Impact:** Prevents full infrastructure cleanup
   - **Status:** ⚠️ likely resolves after manual ECR deletion and secret scheduling expires

#### Remaining Tasks (Phase 1):
- [ ] Option A (Recommended): Wait 5-10 minutes for deletion scheduling to expire, then retry terraform destroy + apply
- [ ] Option B (Manual cleanup): 
  - [ ] `aws ecr delete-repository --repository-name eyedar-prod-dashboard --force`
  - [ ] `aws secretsmanager restore-secret --secret-id eyedar-prod-db-password-v3` (cancel deletion)
  - [ ] Retry terraform destroy
- [ ] Run fresh `terraform destroy` to remove all infrastructure
- [ ] Run `terraform apply` to rebuild from scratch
- [ ] Capture all created resource ARNs and IDs

