# Project Review: AWS-SERVER (Eyedar System) — Clean Audit v3
### Focus: Critical blockers + code issues only. File organization cleaned.
**Date: 2026-03-24**

---

## CLEANUP COMPLETED ✅

The following irrelevant files have been removed to streamline the project:

| Category | Deleted | Reason |
|----------|---------|--------|
| **Security Risk** | `run_migration.py`, `task-def-migration.json` | Plaintext DB password in committed files |
| **Config Drift** | `iam-policy.json`, `iam-policy-clean.json`, `waf-*.json` | Manual CLI artifacts; not referenced by Terraform |
| **Stale Planning** | `Cloud_Setup.md`, `terraform_setup.md`, `VALIDATION_REPORT.md` | Pre-implementation designs; contradicted by actual code |
| **Obsolete Processes** | `RUN-MIGRATION-MANUAL.md`, `STATUS.md` | Manual processes; no longer needed |
| **Out of Scope** | `PROMPT-APP-INTEGRATION.md` | Belongs in Flutter app repo, not AWS infra |
| **Build Artifacts** | `infra/envs/prod/tfplan` | Stale binary snapshot |

**Files Organized into Structure:**
- `scripts/` — Utility scripts: `check_logs.py`, `read_logs.py`, `read_api_logs.py`, `push-to-github.ps1`, `cloudshell-migration.sh`
- `docs/ops/` — Operational guides: `DEPLOYMENT.md`, `RESTART.md`, `ECR.md`, `FIREBASE.md`

---

## PART 1: CRITICAL BLOCKERS — FIX THESE FIRST

### 1. Missing `00-foundation/secrets` Terraform Module
**Severity: CRITICAL — Terraform init fails**
**File:** `infra/envs/prod/main.tf:26-32`

```hcl
module "secrets" {
  source = "../../modules/00-foundation/secrets"  # ← THIS DIRECTORY DOES NOT EXIST
  ...
}
```

**Why:** 8+ downstream modules depend on outputs from this module (`db_secret_arn`, `firebase_secret_arn`, `api_keys_secret_arn`). Terraform cannot initialize without it.

**What needs to be created:**
- `infra/modules/00-foundation/secrets/main.tf`
- `infra/modules/00-foundation/secrets/variables.tf`
- `infra/modules/00-foundation/secrets/outputs.tf`

**Minimum resources:**
```hcl
resource "aws_secretsmanager_secret" "db" {
  name = "eyedar-prod-db-password"
  # Do NOT set secret_string here — value goes in Secrets Manager console, fetched at runtime
}

resource "aws_secretsmanager_secret" "firebase" {
  name = "eyedar-prod-firebase-key"
}

resource "aws_secretsmanager_secret" "api_keys" {
  name = "eyedar-prod-api-keys"
}

output "db_secret_arn" { value = aws_secretsmanager_secret.db.arn }
output "firebase_secret_arn" { value = aws_secretsmanager_secret.firebase.arn }
output "api_keys_secret_arn" { value = aws_secretsmanager_secret.api_keys.arn }
```

---

### 2. RDS Backups Disabled — Zero Recovery Path
**Severity: CRITICAL — Data loss is permanent**
**File:** `infra/envs/prod/main.tf:99`

```hcl
backup_retention_days = 0  # ← DISABLES ALL AUTOMATED BACKUPS
```

**Impact:** Database failure, accidental `DROP TABLE`, or corruption = permanent data loss. No recovery snapshot exists.

**Fix:** Change to `backup_retention_days = 7` (backup storage costs ~$1-2/month at this scale)

---

### 3. HTTPS Disabled — Location Data Sent Over Plain HTTP
**Severity: CRITICAL — Unencrypted GPS coordinates**
**File:** `infra/envs/prod/main.tf:254-261` (commented out)

```hcl
# module "acm" { ... }  # ← COMMENTED OUT
# acm_certificate_arn = ""  # ← EMPTY
```

**Impact:**
- Driver GPS locations sent unencrypted: `POST /api/v1/location`
- API keys sent plaintext in `X-API-Key` header
- Firebase tokens transmitted without encryption

**Fix:** Uncomment the ACM module and set `acm_certificate_arn` in the ALB module

---

### 4. Missing `00-foundation/secrets` Module Outputs — 8 Modules Blocked
**Severity: CRITICAL**

These modules expect secrets ARN outputs that don't exist:
- `ecs_task_roles` (line 26)
- `ecs_services` (line 32)
- Others downstream

Cannot proceed until the secrets module is created.

---

## PART 2: INFRASTRUCTURE MISCONFIGURATION — Free-Tier Settings in Production

### 5. RDS Instance Too Small — 1 GB RAM, PostGIS Will OOM
**Severity: HIGH**
**File:** `infra/envs/prod/variables.tf:72`

```hcl
rds_instance_class = "db.t4g.micro"  # 1 GB RAM
```

With PostGIS + concurrent workload (ESP devices writing, mobile apps reading, geospatial queries), 1 GB will cause:
- Out-of-memory kills
- Swap thrashing
- Query plan degradation

**Fix:** Change to `db.t4g.medium` (4 GB RAM, ~$60/mo). Minimum viable for PostGIS production.

---

### 6. RDS Single-AZ — One Failure Stops the Database
**Severity: HIGH**
**File:** `infra/envs/prod/variables.tf:83`

```hcl
rds_multi_az = false
```

Any AZ failure or AWS maintenance = full database downtime. For a road safety system, unacceptable.

**Fix:** Change to `rds_multi_az = true` (~$60/mo additional cost for failover replica)

---

### 7. Redis Single Node — No Failover, Data Loss on Failure
**Severity: MEDIUM-HIGH**
**File:** `infra/envs/prod/variables.tf:95`

```hcl
redis_num_cache_nodes = 1
```

Single node failure = all cached state lost, all sessions lost, service disruption until cache rebuilds.

**Fix:** Change to `redis_num_cache_nodes = 2` (replication + automatic failover)

---

### 8. Redis Instance Too Small — 0.5 GB RAM
**Severity: MEDIUM**
**File:** `infra/envs/prod/variables.tf:91`

```hcl
redis_node_type = "cache.t4g.micro"  # 0.5 GB
```

Insufficient for session storage + rate limiting + application caching. Will hit limits as user count grows.

**Fix:** Change to `cache.t4g.small` (1.37 GB, ~$25/mo)

---

### 9. Single NAT Gateway — AZ-Level Outage Risk
**Severity: MEDIUM**
**File:** `infra/envs/prod/variables.tf:43`

```hcl
nat_gateway_count = 1
```

NAT lives in one AZ. If that AZ has issues, all private tasks lose internet (cannot pull images, reach Secrets Manager, contact Firebase).

**Fix:** Change to `nat_gateway_count = 2` (one per AZ, ~$32/mo)

---

### 10. Container Insights Disabled — No Per-Task Metrics
**Severity: MEDIUM**
**File:** `infra/envs/prod/variables.tf:172`

```hcl
enable_container_insights = false
```

No CPU/memory metrics per service, no autoscaling decisions possible.

**Fix:** Change to `enable_container_insights = true` (~$0.50/task/mo)

---

### 11. Monthly Budget Set to $100 — Will Trigger Immediately
**Severity: MEDIUM**
**File:** `infra/envs/prod/variables.tf:176`

```hcl
monthly_budget_amount = 100
```

Current infrastructure costs ~$300-350/mo. Budget fires on day 1, provides no useful signal.

**Fix:** Change to `350` (adjust after 2 months of actual billing)

---

### 12. All Services Use `:latest` Image Tag — No Rollback
**Severity: MEDIUM**
**File:** `infra/envs/prod/variables.tf:101-129`

```hcl
image_tag = "latest"  # All services
```

ECS task restarts pull whatever was pushed last. No version pinning, no rollback path.

**Fix:** Use git commit SHA tags (e.g., `sha-a1b2c3d`) passed from CI/CD, not `:latest` defaults

---

## PART 3: MISSING SERVICE CODE — Core Product Features Not Implemented

### 13. Detection Endpoint Missing
**Severity: CRITICAL — Core feature incomplete**
**Missing:** `POST /api/v1/detections`

ESP devices cannot submit animal detections. The entire detection → worker pipeline has no entry point.

**Required:** Add endpoint to `workers/api/src/index.js`
- Validate detection payload (timestamp, location, image URLs, sensor ID)
- Publish to SQS `detection_created` queue
- Return 200 OK or error response

---

### 14. `worker-ingest` Service Missing
**Severity: CRITICAL — Pipeline broken**
**Missing:** `workers/worker-ingest/` (entire directory)

No code exists to consume from SQS `detection_created`, persist to RDS, and publish to `verify_requested` queue.

**Required:** New service
- Read from SQS `detection_created`
- Validate detection, check for duplicates
- Insert into `detections` table
- Publish to SQS `verify_requested`
- Delete from queue on success

---

### 15. `worker-verify` Service Missing
**Severity: HIGH — AI verification step missing**
**Missing:** `workers/worker-verify/` (entire directory)

No code exists to consume from `verify_requested`, run AI model, and publish verified results.

**Required:** New service (or can be placeholder for now)
- Read from SQS `verify_requested`
- Call external AI verification API (or mock for testing)
- Publish to SQS `verified_animals`
- Update detection record with result

---

### 16. `dashboard` Service Missing
**Severity: HIGH — Admin view missing**
**Missing:** `workers/dashboard/` (entire directory)

No code exists. ECS service definition references code that doesn't exist; service will crash loop.

**Required:** New service
- Serve static web dashboard
- Connected to API via ALB routing
- CSP headers must reference ALB DNS, not docker-compose internal hostnames

---

### 17. `scheduler` Service Missing
**Severity: MEDIUM — Periodic tasks**
**Missing:** `workers/scheduler/` (may not be needed yet)

EventBridge Scheduler can trigger this, but no code exists. Can defer; optional early on.

---

## PART 4: CODE SECURITY ISSUES

### 18. API Leaks Database Schema in Error Responses
**Severity: HIGH — Information disclosure**
**File:** `workers/api/src/index.js:139, 193, 237`

```javascript
res.status(500).json({ details: error.message });  // Exposes PostgreSQL errors
```

PostgreSQL errors leak table/column names, query structure, sometimes data samples.

**Fix:** Replace with generic message
```javascript
res.status(500).json({ error: "Server error" });
// Log error.message to CloudWatch, not to client
```

---

### 19. Notification Endpoint Has No Ownership Check (IDOR)
**Severity: HIGH — Broken authorization**
**File:** `workers/api/src/index.js:202`

```javascript
app.get('/api/notifications', (req, res) => {
  const user_id = req.query.user_id;  // ← Client provides user_id, no verification
  // ... returns notifications for ANY user_id
});
```

Any valid API key can read any user's notification history. The endpoint takes `user_id` from query string without verifying the caller owns that account.

**Fix:** Extract user identity from API key or add verified user_id on the request
```javascript
const user_id = req.user.id;  // From authenticated context, not query param
```

---

### 20. CORS Allows All Origins
**Severity: MEDIUM-HIGH**
**File:** `workers/api/src/index.js:21`

```javascript
app.use(cors());  // Allows any origin
```

If a web dashboard or mobile web frontend calls this API, any website can make cross-origin requests with user credentials.

**Fix:**
```javascript
app.use(cors({
  origin: process.env.ALLOWED_ORIGINS?.split(',') || [],
  credentials: true
}));
```

---

### 21. API Boots with Missing API Key — Silent Broken State
**Severity: MEDIUM**
**File:** `workers/api/src/index.js:265`

```javascript
Secrets.getSecretValue().catch(() => {
  console.warn("Could not load API key");  // ← Just logs, doesn't fail
});
// ... server boots anyway
```

If Secrets Manager is unavailable, the API boots successfully. All requests return 503. ALB health check passes (no auth required). Service appears healthy but is broken.

**Fix:** Fail hard
```javascript
Secrets.getSecretValue().catch((err) => {
  console.error("Fatal: Could not load API key", err);
  process.exit(1);  // Let ECS restart the task
});
```

---

### 22. Duplicate API Key Secrets in Secrets Manager
**Severity: MEDIUM — Hidden broken state**
**Files:** Secrets Manager config (not in repo anymore, but confirmed in audit)

Two secrets exist:
- `eyedar-prod-api-keys-WIW2l9` (random suffix)
- `eyedar-prod/api-key-YijVfv` (random suffix)

Code loads `eyedar-prod/api-key`. If the other one gets populated by mistake, the API boots silently broken.

**Fix:** Delete the unused secret, keep one canonical name

---

### 23. `worker-notify` Uses Deprecated AWS SDK v2
**Severity: MEDIUM — EOL, no security patches**
**File:** `workers/worker-notify/package.json`

```javascript
const AWS = require('aws-sdk');  // SDK v2, EOL Nov 2023
```

The API correctly uses SDK v3. Mixing major versions creates maintenance debt.

**Fix:** Upgrade to SDK v3
```javascript
import { SQSClient, receive_message } from '@aws-sdk/client-sqs';
```

---

## PART 5: INFRASTRUCTURE CODE ISSUES

### 24. Deploy Policies Module Missing
**Severity: HIGH — CI/CD non-functional**
**Missing:** `infra/modules/60-cicd/deploy_policies/`

GitHub Actions OIDC role exists, but the actual IAM policies (ECR push, ECS update) are not created. CI/CD cannot deploy.

**Required:** Create module to attach policies to the OIDC role:
- `ecr:GetDownloadUrlForLayer`, `ecr:PutImage`, etc.
- `ecs:UpdateService`, `ecs:DescribeServices`, etc.

---

### 25. Dashboard Port Mismatch
**Severity: MEDIUM — Health checks fail**
**Issue:** ALB routes to `module.ecs_services.dashboard_port`. If task definition uses port 3000 and Dockerfile listens on port 80, health checks fail.

**Fix:** Ensure port consistency across Terraform variables, task definition, and Dockerfile

---

### 26. Dashboard CSP Headers Reference Docker Hostnames
**Severity: MEDIUM — Browsers block API calls**
**File:** `Dockerfile.dashboard` (from local server)

```
connect-src 'self' http://rust_api:8080 ws://rust_api:8080 https://*.ts.net
```

- `rust_api:8080` — internal docker-compose name, doesn't exist in AWS
- `*.ts.net` — Tailscale VPN, irrelevant in AWS

**Fix:** Update CSP headers to reference ALB DNS name
```
connect-src 'self' https://api.yourdomain.com wss://api.yourdomain.com
```

---

### 27. `worker-notify` Saves Detections (Wrong Responsibility)
**Severity: MEDIUM — Future duplicate issue**
**File:** `workers/worker-notify/src/index.js:211-235`

Worker-notify saves detections to DB, but that's `worker-ingest`'s job. When `worker-ingest` is built, attempts to insert the same detection will cause conflicts (mitigated by `ON CONFLICT DO NOTHING`, but blurs responsibility).

**Fix:** Remove `saveDetection()` from worker-notify once worker-ingest exists

---

## PART 6: CONFIGURATION RECOMMENDATIONS

### 28. Missing VPC Endpoint Configuration
**Severity: LOW-MEDIUM**
**File:** `infra/envs/prod/variables.tf:52`

```hcl
enable_interface_endpoints = false
```

All Secrets Manager, CloudWatch Logs, and ECR calls route through NAT, adding cost and latency.

**Recommendation:** Enable for at least Secrets Manager and CloudWatch Logs (~$44/mo for 3 endpoints across 2 AZs, but saves NAT traffic)

---

## PART 7: WHAT'S WORKING WELL ✅

These are done correctly and should not change:

- **Terraform module structure** — foundation → network → data → compute → edge → cicd
- **PostGIS geospatial schema** — correct `GEOGRAPHY(POINT, 4326)`, GIST indexes, `ST_DWithin` queries
- **Non-root users in Fargate Dockerfiles** — security best practice
- **`crypto.timingSafeEqual`** in API — prevents timing-based key enumeration
- **SQS + Dead Letter Queue pattern** — at-least-once delivery with poison isolation
- **S3 VPC Gateway Endpoint** — keeps S3 traffic private, reduces NAT cost
- **KMS encryption** on RDS, S3, Secrets Manager
- **SIGTERM graceful shutdown** in workers
- **SQS long polling (20s)** — reduces API calls
- **Health check endpoints** in existing services
- **Helmet.js security headers** on API
- **FCM severity logic** (danger/warning/info by distance)
- **Migration script pattern** — `cloudshell-migration.sh` correctly fetches password from Secrets Manager at runtime

---

## PART 8: PRIORITIZED FIX LIST

### Phase 1: Unblock Terraform (Hours 1-2)
1. Create `infra/modules/00-foundation/secrets/` module
2. Fix `backup_retention_days = 7` in `main.tf:99`
3. Run `terraform plan` to validate

### Phase 2: Infrastructure Configuration (Hours 3-4)
4. Uncomment ACM module, enable HTTPS
5. Upgrade RDS: `db.t4g.micro` → `db.t4g.medium`
6. Set `rds_multi_az = true`
7. Upgrade Redis: `cache.t4g.micro` → `cache.t4g.small`, set `redis_num_cache_nodes = 2`
8. Set `nat_gateway_count = 2`
9. Set `enable_container_insights = true`
10. Update budget: `100` → `350`
11. Disable `:latest` tags, use git SHA
12. Create `infra/modules/60-cicd/deploy_policies/` module
13. Run `terraform plan` again, review changes

### Phase 3: Code Security Fixes (Hours 5-6)
14. Fix API error responses (hide DB schema)
15. Fix notification endpoint (add ownership check)
16. Fix CORS (restrict origins)
17. Fix API startup (fail hard without API key)
18. Upgrade worker-notify to SDK v3

### Phase 4: Implement Missing Services (Hours 7+)
19. Create `POST /api/v1/detections` endpoint
20. Implement `workers/worker-ingest/` service
21. Implement `workers/dashboard/` service (with correct CSP headers)
22. Implement `workers/worker-verify/` service (can be placeholder)
23. Create `deploy_policies` IAM policy attachment

### Phase 5: Pre-Deployment Testing
24. Build and push all container images
25. Run migration with new secrets approach
26. Deploy to ECS
27. Test detection → ingest → verify → notify pipeline end-to-end
28. Verify HTTPS on ALB
29. Load test with realistic device/location/notification volume

---

## Summary

| Category | Count | Blocker? | Item |
|----------|-------|----------|------|
| Critical blockers | 4 | YES | Missing secrets module, no backups, no HTTPS, missing endpoints |
| Infrastructure misconfig | 8 | YES | Undersized RDS/Redis, single AZ, single NAT, free-tier settings |
| Code security issues | 6 | YES | Error leaks, IDOR, CORS, broken startup, SDK v2, duplicates |
| Missing services | 4 | YES | worker-ingest, worker-verify, dashboard, scheduler |
| Infra code gaps | 3 | YES | No deploy policies, port mismatch, CSP headers wrong |
| Configuration | 1 | NO | VPC endpoints disabled (optimization, not blocker) |
| **Things working** | **10+** | — | Terraform structure, PostGIS schema, security headers, graceful shutdown |

**Estimated effort:** 24-32 hours total to Phase 4 (deployable state). Phase 5 (testing) depends on load test scope.

---

## Next Step

**Start with Phase 1:** Create the missing secrets module. This unblocks Terraform and allows the rest of the fixes to proceed systematically.
