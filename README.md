
# AWS-SERVER

Eyedar is an AWS-hosted animal detection pipeline. The system accepts detection events from the Rust API, stores images in S3, writes records to PostgreSQL, moves work through SQS, and eventually verifies detections and notifies users.

## What Lives Here

- `infra/` contains the Terraform infrastructure for AWS.
- `services/rust_api/` contains the Rust entry point that receives alert uploads.
- `workers/worker-ingest/` consumes new detections, persists them, and forwards work for verification.
- `workers/worker-verify/` will fetch S3 images and run YOLO-based verification.
- `workers/worker-notify/` sends push notifications after verified detections.
- `workers/api/` contains the legacy Node API path used in earlier phases.

## Current Flow

1. A device or app submits an alert to the Rust API.
2. The API stores the image in S3 and creates the detection record in RDS.
3. The API publishes a message to `eyedar-prod-detection-created`.
4. `worker-ingest` consumes that message, writes/normalizes the record, and publishes a verification request.
5. `worker-verify` fetches the image from S3, runs YOLO, and routes the result.
6. `worker-notify` sends notifications to the app when a detection is verified.

## Repository Layout

```text
infra/               Terraform root, envs, modules, IAM policies
services/rust_api/   Rust API service and container build
services/mqtt-monitor/ Optional Rust service for monitoring
workers/api/         Legacy Node API service
workers/worker-ingest/  SQS ingest worker
workers/worker-verify/  YOLO verification worker
workers/worker-notify/  FCM notification worker
docs/ops/            Operational notes and deployment guides
```

## Canonical Docs

- [Deployment Guide](docs/ops/DEPLOYMENT.md)
- [Project Review](PROJECT_REVIEW.md)
- [Terraform Setup Reference](terraform_setup.pdf)

Keep these as the source of truth for deployment steps, current status, and infrastructure expectations.

## Prerequisites

- AWS CLI configured for the target account
- Terraform 1.7+ installed
- Docker installed
- Git installed
- Firebase service account key available for notification setup

## Bootstrap GitHub OIDC

Before the first Terraform or deployment run, create or update the GitHub Actions OIDC role used by the pipeline:

```powershell
$AWS_REGION = "us-east-1"

aws iam create-open-id-connect-provider `
  --url https://token.actions.githubusercontent.com `
  --client-id-list sts.amazonaws.com `
  --thumbprint-list 6938fd4d98bab03faadb97b34396831e3780aea1 `
  --region $AWS_REGION

aws iam update-assume-role-policy `
  --role-name eyedar-prod-github-actions-deployer `
  --policy-document file://infra/iam/github-oidc-trust-policy.json `
  --region $AWS_REGION
```

Set the repository variable `AWS_ACCOUNT_ID_A` to your AWS account ID.

## Local Work

Useful checks while developing:

```powershell
terraform -chdir=infra/envs/prod plan
terraform -chdir=infra/envs/prod validate
```

## Deployment

The GitHub Actions workflow in `.github/workflows/deploy.yml` builds the service images, pushes them to ECR, and triggers ECS deployments for the main runtime services.

If you need the operational walk-through, use [docs/ops/DEPLOYMENT.md](docs/ops/DEPLOYMENT.md).

## Next Work

- Finish API verification in the current phase.
- Implement YOLO in `worker-verify`.
- Complete notification and app messaging updates.
- Refresh the worker and infra READMEs to match the current architecture.

## Status

The project is in an active cleanup and feature-completion phase. The infrastructure exists, the ingest path is validated, and the remaining work is to complete verification, notifications, and the user-facing documentation.
