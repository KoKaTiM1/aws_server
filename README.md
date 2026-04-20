# DAR - Document Analysis Road Safety

**Wildlife Collision Prevention System**

Real-time animal detection and driver notification system using AWS infrastructure, Flutter mobile app, and Firebase Cloud Messaging.

## Table of Contents

- [Overview](#overview)
- [System Architecture](#system-architecture)
- [Project Status](#project-status)
- [Infrastructure Components](#infrastructure-components)
- [Services](#services)
- [Getting Started](#getting-started)
- [Deployment](#deployment)
- [Cost Management](#cost-management)
- [Next Steps](#next-steps)

## Overview

DAR is a comprehensive wildlife collision prevention system that:
1. **Detects animals** near roads using AI/computer vision
2. **Verifies detections** through a validation pipeline
3. **Calculates proximity** to nearby drivers using PostGIS geospatial queries
4. **Sends real-time notifications** to drivers via Firebase Cloud Messaging (FCM)
5. **Manages user data** including routes, preferences, and notification history

## System Architecture

```
ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”
ג”‚                        Client Layer                              ג”‚
ג”‚  ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”         ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”              ג”‚
ג”‚  ג”‚  Flutter Mobile  ג”‚ג†ג”€ FCM ג”€ג†’ג”‚  Firebase Admin  ג”‚              ג”‚
ג”‚  ג”‚      App         ג”‚         ג”‚       SDK        ג”‚              ג”‚
ג”‚  ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜         ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜              ג”‚
ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”¬ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”¬ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜
             ג”‚                            ג”‚
             ג”‚ HTTPS                      ג”‚ Notifications
             ג†“                            ג†“
ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”
ג”‚                     AWS Infrastructure                           ג”‚
ג”‚                                                                  ג”‚
ג”‚  ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”   ג”‚
ג”‚  ג”‚                  Application Load Balancer               ג”‚   ג”‚
ג”‚  ג”‚              eyedar-prod-alb-*.elb.amazonaws.com         ג”‚   ג”‚
ג”‚  ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜   ג”‚
ג”‚                            ג”‚                                     ג”‚
ג”‚                            ג†“                                     ג”‚
ג”‚  ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”  ג”‚
ג”‚  ג”‚                    ECS Fargate Cluster                    ג”‚  ג”‚
ג”‚  ג”‚  ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”              ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”    ג”‚  ג”‚
ג”‚  ג”‚  ג”‚   API Service  ג”‚              ג”‚  Worker-Notify   ג”‚    ג”‚  ג”‚
ג”‚  ג”‚  ג”‚   (Node.js)    ג”‚              ג”‚    (Node.js)     ג”‚    ג”‚  ג”‚
ג”‚  ג”‚  ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜              ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜    ג”‚  ג”‚
ג”‚  ג”‚         ג”‚                                  ג”‚              ג”‚  ג”‚
ג”‚  ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”¼ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”¼ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜  ג”‚
ג”‚            ג”‚                                  ג”‚                  ג”‚
ג”‚            ג†“                                  ג†“                  ג”‚
ג”‚  ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”              ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”        ג”‚
ג”‚  ג”‚   RDS PostgreSQL ג”‚              ג”‚   SQS Queues     ג”‚        ג”‚
ג”‚  ג”‚   + PostGIS      ג”‚              ג”‚   (4 queues)     ג”‚        ג”‚
ג”‚  ג”‚   db.t4g.micro   ג”‚              ג”‚ - verified       ג”‚        ג”‚
ג”‚  ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜              ג”‚ - raw            ג”‚        ג”‚
ג”‚                                    ג”‚ - dlq_verified   ג”‚        ג”‚
ג”‚  ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”              ג”‚ - dlq_raw        ג”‚        ג”‚
ג”‚  ג”‚  ElastiCache     ג”‚              ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜        ג”‚
ג”‚  ג”‚     Redis        ג”‚                                           ג”‚
ג”‚  ג”‚  cache.t4g.micro ג”‚              ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”        ג”‚
ג”‚  ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜              ג”‚ Secrets Manager  ג”‚        ג”‚
ג”‚                                    ג”‚ - DB Credentials ג”‚        ג”‚
ג”‚  ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”              ג”‚ - Firebase Key   ג”‚        ג”‚
ג”‚  ג”‚   S3 Buckets     ג”‚              ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜        ג”‚
ג”‚  ג”‚ - images         ג”‚                                           ג”‚
ג”‚  ג”‚ - videos         ג”‚              ג”ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”        ג”‚
ג”‚  ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜              ג”‚  CloudWatch Logs ג”‚        ג”‚
ג”‚                                    ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜        ג”‚
ג””ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”€ג”˜
```

## Project Status

### Completed (As of February 16, 2026)

#### Infrastructure (Terraform)
- [x] **VPC & Networking** - Private/public subnets, NAT Gateway, Internet Gateway
- [x] **Security Groups** - Configured for ALB, ECS, RDS, Redis
- [x] **Application Load Balancer** - Health checks, SSL-ready
- [x] **ECS Cluster** - Fargate launch type
- [x] **RDS PostgreSQL 15.7** - db.t4g.micro, 20GB storage
- [x] **ElastiCache Redis** - cache.t4g.micro for session management
- [x] **S3 Buckets** - Images, videos, backups (3 buckets)
- [x] **SQS Queues** - 4 queues (verified, raw, 2x DLQ)
- [x] **ECR Repositories** - 5 repos for Docker images
- [x] **KMS Keys** - Encryption for secrets and logs
- [x] **CloudWatch** - Log groups for all services
- [x] **IAM Roles** - Task roles with least-privilege policies
- [x] **Secrets Manager** - DB credentials, Firebase service account

**Total Deployed Resources:** 115

#### Worker-Notify Service
- [x] **Node.js Application** - 370+ lines
- [x] **Firebase Admin SDK** - Integrated for FCM notifications
- [x] **PostGIS Integration** - Geospatial distance calculations
- [x] **SQS Consumer** - Long polling, visibility timeout handling
- [x] **Docker Image** - Built and pushed to ECR
- [x] **ECS Task Definition** - Fargate, 256 CPU, 512 MB RAM
- [x] **Environment Variables** - DB, SQS, Firebase configured
- [x] **Secret Management** - Pulling from AWS Secrets Manager

#### Configuration & Documentation
- [x] **Terraform Validated** - All modules tested
- [x] **Cost Analysis** - ~$91/month (without ECS), ~$243/month (full operation)
- [x] **Firebase Credentials** - Uploaded to Secrets Manager
- [x] **RDS Password** - Generated and configured
- [x] **Migration Scripts** - SQL ready with PostGIS
- [x] **Manual Guides** - RESTART-SERVICES.md, RUN-MIGRATION-MANUAL.md

### Pending (Next Session)

#### Database
- [ ] **Run DB Migration** - Execute `001_init_schema.sql` via AWS RDS Query Editor
  - Install PostGIS extension
  - Create tables: users, animal_detections, notifications, user_routes
  - Create geospatial indexes
  - Create `find_nearby_users()` stored procedure

#### Testing
- [ ] **End-to-End Test** - Send test detection through pipeline
- [ ] **FCM Notification Test** - Verify Flutter app receives alerts
- [ ] **Load Testing** - SQS queue processing performance
- [ ] **Geospatial Accuracy** - Validate distance calculations

#### API Service
- [ ] **Deploy API Docker Image** - Build and push to ECR
- [ ] **Configure API Routes** - User management, detection endpoints
- [ ] **ALB Target Group** - Connect API service to load balancer

#### Monitoring & Alerts
- [ ] **CloudWatch Alarms** - CPU, memory, error rate thresholds
- [ ] **SNS Topics** - Email/SMS notifications for critical issues
- [ ] **X-Ray Integration** - Distributed tracing

## Infrastructure Components

### Networking (VPC)
- **CIDR:** 10.0.0.0/16
- **Public Subnets:** 2 AZs (us-east-1a, us-east-1b)
- **Private Subnets:** 2 AZs (isolated for ECS, RDS)
- **NAT Gateway:** High availability
- **Internet Gateway:** Public traffic routing

### Compute (ECS Fargate)
- **Cluster:** eyedar-prod
- **Services:**
  - `eyedar-prod-api` (pending deployment)
  - `eyedar-prod-worker-notify` (deployed, needs DB)
- **Task Definitions:** Auto-scaling ready
- **Launch Type:** FARGATE (serverless containers)

### Database (RDS PostgreSQL)
- **Version:** 15.7
- **Instance:** db.t4g.micro (2 vCPU, 1 GB RAM)
- **Storage:** 20 GB SSD
- **Backup:** 0 days retention (Free Tier)
- **Extensions:** PostGIS (pending installation)
- **Stop Limit:** 7 days before auto-restart

### Cache (ElastiCache Redis)
- **Version:** 7.0
- **Instance:** cache.t4g.micro (2 vCPU, 0.5 GB RAM)
- **Use Case:** Session storage, rate limiting

### Storage (S3)
- **eyedar-prod-images:** Animal detection images
- **eyedar-prod-videos:** Video footage (if applicable)
- **eyedar-prod-backups:** Database backups
- **Lifecycle Policies:** Auto-archive to Glacier after 90 days

### Messaging (SQS)
- **eyedar-prod-verified-animals:** Main queue for Worker-Notify
- **eyedar-prod-raw-detections:** Pre-validation queue
- **eyedar-prod-dlq-verified:** Dead Letter Queue
- **eyedar-prod-dlq-raw:** Dead Letter Queue
- **Visibility Timeout:** 300 seconds
- **Message Retention:** 14 days

### Security
- **KMS Key:** Encrypts CloudWatch Logs, Secrets Manager
- **Secrets Manager:**
  - `eyedar-prod-db-credentials` (username/password)
  - `eyedar-prod-firebase-key` (Firebase Admin SDK service account)
- **IAM Roles:** Separate roles per service (least privilege)
- **Security Groups:** Port-specific rules (5432 for RDS, 6379 for Redis)

## Services

### 1. Worker-Notify

**Purpose:** Polls SQS queue for verified animal detections, calculates nearby drivers, sends FCM notifications.

**Tech Stack:**
- Node.js 18
- Firebase Admin SDK
- PostgreSQL client (pg) with PostGIS
- AWS SDK v2 (SQS, Secrets Manager)

**Key Features:**
- Long polling (20 seconds wait time)
- Batch processing (up to 10 messages)
- Distance-based severity levels (danger/warning/info)
- Estimated time to collision calculation
- Automatic message deletion on success
- Error handling with DLQ

**Environment Variables:**
- `DB_HOST`, `DB_PORT`, `DB_NAME`
- `DB_USERNAME`, `DB_PASSWORD` (from Secrets Manager)
- `SQS_QUEUE_URL_VERIFIED_ANIMALS`
- `FIREBASE_SERVICE_ACCOUNT` (from Secrets Manager)

**Docker Image:**
- Size: ~682 MB
- Base: node:18-alpine
- User: nodejs (non-root)
- Port: N/A (background worker)

### 2. API Service (Pending)

**Purpose:** REST API for user management, detection uploads, route management.

**Planned Endpoints:**
- `POST /api/users` - Register user
- `GET /api/users/:id` - Get user profile
- `POST /api/detections` - Upload detection
- `GET /api/notifications` - User notification history
- `POST /api/routes` - Save user route

## נ› ן¸ Getting Started

### Prerequisites

- **AWS CLI v2** - [Install](https://aws.amazon.com/cli/)
- **Terraform v1.7+** - [Install](https://www.terraform.io/downloads)
- **Docker Desktop** - [Install](https://www.docker.com/products/docker-desktop)
- **Node.js 18+** - [Install](https://nodejs.org/)
- **Git** - [Install](https://git-scm.com/)

### AWS Configuration

```bash
aws configure
# AWS Access Key ID: [your-access-key]
# AWS Secret Access Key: [your-secret-key]
# Default region: us-east-1
# Default output format: json
```

### Bootstrap: Create OIDC Provider and GitHub Actions Role

Before deploying infrastructure with Terraform, set up the OIDC provider and IAM role for GitHub Actions:

```bash
# 1. Create OIDC Provider for GitHub Actions
aws iam create-open-id-connect-provider \
  --url https://token.actions.githubusercontent.com \
  --client-id-list sts.amazonaws.com \
  --thumbprint-list 6938fd4d98bab03faadb97b34396831e3780aea1 1c58a3a8518e8759bf075b76b750d4f2df264fcd \
  --region us-east-1

# 2. Create IAM Role for GitHub Actions deployer
aws iam create-role \
  --role-name eyedar-prod-github-actions-deployer \
  --assume-role-policy-document '{
    "Version": "2012-10-17",
    "Statement": [
      {
        "Effect": "Allow",
        "Principal": {
          "Federated": "arn:aws:iam::YOUR_ACCOUNT_ID:oidc-provider/token.actions.githubusercontent.com"
        },
        "Action": "sts:AssumeRoleWithWebIdentity",
        "Condition": {
          "StringLike": {
            "token.actions.githubusercontent.com:sub": "repo:YOUR_ORG/YOUR_REPO:ref:refs/heads/main"
          }
        }
      }
    ]
  }' \
  --region us-east-1

# 3. Attach AdministratorAccess policy to the role
aws iam attach-role-policy \
  --role-name eyedar-prod-github-actions-deployer \
  --policy-arn arn:aws:iam::aws:policy/AdministratorAccess \
  --region us-east-1
```

**Note:** Replace `YOUR_ACCOUNT_ID`, `YOUR_ORG`, and `YOUR_REPO` with your actual values.

### Clone Repository

```bash
git clone https://github.com/eye-dar/AWS-server.git
cd AWS-server
```

## Deployment

### 1. Infrastructure (Terraform)

```bash
cd infra/envs/prod
terraform init
terraform plan
terraform apply
```

**Note:** This will create ~115 AWS resources. Estimated cost: $91-243/month.

### 2. Upload Firebase Credentials

```powershell
$firebaseKey = Get-Content "path/to/firebase-adminsdk.json" -Raw
aws secretsmanager put-secret-value `
  --secret-id eyedar-prod-firebase-key `
  --secret-string $firebaseKey `
  --region us-east-1
```

### 3. Build & Push Worker-Notify

```bash
cd workers/worker-notify

# Build Docker image
docker build -t eyedar-worker-notify:latest .

# Tag for ECR
docker tag eyedar-worker-notify:latest \
  221671810590.dkr.ecr.us-east-1.amazonaws.com/eyedar-prod-worker-notify:latest

# Login to ECR
aws ecr get-login-password --region us-east-1 | \
  docker login --username AWS --password-stdin \
  221671810590.dkr.ecr.us-east-1.amazonaws.com

# Push to ECR
docker push 221671810590.dkr.ecr.us-east-1.amazonaws.com/eyedar-prod-worker-notify:latest

# Update ECS service
aws ecs update-service \
  --cluster eyedar-prod \
  --service eyedar-prod-worker-notify \
  --force-new-deployment \
  --region us-east-1
```

### 4. Run Database Migration

**Option A: AWS RDS Query Editor (Recommended)**

1. Go to: https://console.aws.amazon.com/rds/home?region=us-east-1#query-editor:
2. Select `eyedar-prod-db`
3. Use Secrets Manager: `eyedar-prod-db-credentials`
4. Copy SQL from `infra/db/migrations/001_init_schema.sql`
5. Execute

**Option B: psql (Local)**

```bash
psql -h eyedar-prod-db.csfmmaq82w8d.us-east-1.rds.amazonaws.com \
     -U eyedar_admin \
     -d eyedar \
     -f infra/db/migrations/001_init_schema.sql
```

## Cost Management

### Monthly Cost Breakdown

| Service | Configuration | Monthly Cost |
|---------|---------------|-------------|
| ECS Fargate (API) | 1 task, 512MB RAM | ~$16 |
| ECS Fargate (Worker) | 1 task, 512MB RAM | ~$16 |
| RDS PostgreSQL | db.t4g.micro, 20GB | ~$16 |
| ElastiCache Redis | cache.t4g.micro | ~$14 |
| ALB | Load balancer + data | ~$18 |
| NAT Gateway | 30GB/month | ~$37 |
| S3 | 10GB storage | ~$0.23 |
| CloudWatch Logs | 5GB/month | ~$2.53 |
| Secrets Manager | 2 secrets | ~$0.80 |
| **Total (Running)** | | **~$120-243/month** |
| **Total (Stopped)** | ALB + NAT only | **~$51/month** |

### Cost Optimization Tips

1. **Stop RDS when not in use** (up to 7 days)
2. **Scale ECS to 0** when testing
3. **Use S3 Lifecycle** policies to archive old data
4. **Enable CloudWatch Logs** retention (7-14 days max)
5. **Delete unused ECR images**

### Shutdown Script

```powershell
# Stop ECS services
aws ecs update-service --cluster eyedar-prod --service eyedar-prod-api --desired-count 0
aws ecs update-service --cluster eyedar-prod --service eyedar-prod-worker-notify --desired-count 0

# Stop RDS (up to 7 days)
aws rds stop-db-instance --db-instance-identifier eyedar-prod-db
```

See [RESTART-SERVICES.md](./RESTART-SERVICES.md) for full shutdown/restart instructions.

## Configuration Files

### Terraform Variables (`infra/envs/prod/terraform.tfvars`)

```hcl
env_name = "prod"
region   = "us-east-1"

vpc_cidr = "10.0.0.0/16"

rds_instance_class = "db.t4g.micro"
redis_node_type    = "cache.t4g.micro"

api_desired_count           = 1
worker_notify_desired_count = 1
```

### Worker Environment (`.env.example`)

```bash
NODE_ENV=production
AWS_REGION=us-east-1

# Database (loaded from Secrets Manager)
DB_HOST=eyedar-prod-db.csfmmaq82w8d.us-east-1.rds.amazonaws.com
DB_PORT=5432
DB_NAME=eyedar

# SQS
SQS_QUEUE_URL_VERIFIED_ANIMALS=https://sqs.us-east-1.amazonaws.com/221671810590/eyedar-prod-verified-animals

# Secrets (loaded automatically by ECS)
# DB_USERNAME (from Secrets Manager)
# DB_PASSWORD (from Secrets Manager)
# FIREBASE_SERVICE_ACCOUNT (from Secrets Manager)
```

## Database Schema

### Tables

#### `users`
- `id` (UUID, Primary Key)
- `username` (VARCHAR, Unique)
- `email` (VARCHAR, Unique)
- `fcm_token` (TEXT) - Firebase Cloud Messaging token
- `phone_number` (VARCHAR)
- `is_active` (BOOLEAN)
- `notification_enabled` (BOOLEAN)
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)

#### `animal_detections`
- `id` (UUID, Primary Key)
- `location` (GEOGRAPHY(POINT, 4326)) - PostGIS point
- `latitude` (DOUBLE PRECISION)
- `longitude` (DOUBLE PRECISION)
- `animal_type` (VARCHAR)
- `confidence` (FLOAT)
- `image_url` (TEXT)
- `detected_at` (TIMESTAMP)
- `verified` (BOOLEAN)
- `created_at` (TIMESTAMP)

**Indexes:**
- Spatial index on `location` (GIST)
- Index on `detected_at`, `verified`

#### `notifications`
- `id` (UUID, Primary Key)
- `user_id` (UUID, Foreign Key ג†’ users)
- `detection_id` (UUID, Foreign Key ג†’ animal_detections)
- `distance_km` (FLOAT)
- `estimated_time_seconds` (INTEGER)
- `severity` (VARCHAR) - danger/warning/info
- `sent_at` (TIMESTAMP)
- `delivered` (BOOLEAN)
- `created_at` (TIMESTAMP)

#### `user_routes`
- `id` (UUID, Primary Key)
- `user_id` (UUID, Foreign Key ג†’ users)
- `route_name` (VARCHAR)
- `start_location` (GEOGRAPHY(POINT, 4326))
- `end_location` (GEOGRAPHY(POINT, 4326))
- `route_path` (GEOGRAPHY(LINESTRING, 4326))
- `created_at` (TIMESTAMP)

### Stored Procedures

#### `find_nearby_users(lat, lon, max_distance_km)`

Returns users within specified distance of a location, ordered by proximity.

**Returns:**
- `user_id`
- `username`
- `fcm_token`
- `distance_km`
- `estimated_time_seconds` (assuming 80 km/h average speed)

## Testing

### Test Detection Flow

1. **Send test message to SQS:**

```bash
aws sqs send-message \
  --queue-url https://sqs.us-east-1.amazonaws.com/221671810590/eyedar-prod-verified-animals \
  --message-body '{
    "id": "test-123",
    "latitude": 32.0853,
    "longitude": 34.7818,
    "animal_type": "deer",
    "confidence": 0.95,
    "timestamp": "2026-02-16T14:30:00Z"
  }' \
  --region us-east-1
```

2. **Check Worker-Notify logs:**

```bash
aws logs tail /ecs/eyedar-prod-worker-notify --follow
```

3. **Verify notification on Flutter app**

## Documentation

- [RESTART-SERVICES.md](./RESTART-SERVICES.md) - Shutdown/restart instructions
- [RUN-MIGRATION-MANUAL.md](./RUN-MIGRATION-MANUAL.md) - Database setup guide
- [infra/README.md](./infra/README.md) - Terraform module documentation
- [workers/worker-notify/README.md](./workers/worker-notify/README.md) - Worker service details

## Useful Links

- **AWS Console:** https://console.aws.amazon.com
- **ECS Cluster:** https://console.aws.amazon.com/ecs/v2/clusters/eyedar-prod
- **RDS Dashboard:** https://console.aws.amazon.com/rds/home?region=us-east-1#database:id=eyedar-prod-db
- **CloudWatch Logs:** https://console.aws.amazon.com/cloudwatch/home?region=us-east-1#logsV2:log-groups
- **API Endpoint:** http://eyedar-prod-alb-334185939.us-east-1.elb.amazonaws.com

## Next Steps

### Immediate (This Week)
1. **Run DB Migration** - Execute SQL via RDS Query Editor
2. **Test Worker-Notify** - Send test SQS message, verify FCM delivery
3. **Deploy API Service** - Build Docker image, configure routes
4. **Configure ALB** - Add API target group

### Short-Term (2-4 Weeks)
1. **Monitoring** - CloudWatch alarms, SNS notifications
2. **Load Testing** - Stress test SQS queue processing
3. **Documentation** - API endpoint documentation (Swagger/OpenAPI)
4. **CI/CD Pipeline** - GitHub Actions for automated deployments

### Long-Term (1-3 Months)
1. **Auto-Scaling** - ECS service auto-scaling based on CPU/memory
2. **Multi-Region** - Disaster recovery setup
3. **Advanced Analytics** - Detection frequency, user engagement metrics
4. **ML Pipeline** - Automated animal type classification improvement

## Team

- **Project:** DAR (Document Analysis Road Safety)
- **Organization:** eye-dar
- **Repository:** https://github.com/eye-dar/AWS-server
- **Region:** us-east-1 (N. Virginia)
- **AWS Account:** 221671810590

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]

## Support

For issues or questions:
- **GitHub Issues:** https://github.com/eye-dar/AWS-server/issues
- **Email:** [Your support email]

---

**Last Updated:** February 16, 2026  
**Status:** Infrastructure deployed, Worker-Notify ready, DB migration pending
