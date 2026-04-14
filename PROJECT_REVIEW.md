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

## Summary

| Item | Status |
|------|--------|
| Core infrastructure | ✅ Deployed |
| Basic services running | ✅ 4 active |
| Image upload end-to-end | ✅ Working |
| Dashboard accessible | ✅ Yes |
| **Next Priority** | **Phase 1: Terraform validation** |
| **Estimated time to publish** | **12-18 hours** |

