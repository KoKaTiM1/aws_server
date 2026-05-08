
# AWS-SERVER

Eyedar is an AWS-hosted animal detection system. The repository contains the Terraform infrastructure, the Rust API entry point, the worker services, and the deployment workflow that builds and pushes the runtime images to AWS.

## What This Project Does

1. Receives detection events through the Rust API.
2. Stores uploaded images in S3.
3. Writes detection records to PostgreSQL.
4. Moves work through SQS for ingest, verification, and notification.
5. Sends push notifications after a detection is verified.

## Key AWS Resources

- S3 bucket: `eyedar-prod-objects-v2`
- ECS cluster: `eyedar-prod`
- GitHub Actions role: `eyedar-prod-github-actions-deployer`
- OIDC provider: `token.actions.githubusercontent.com`
- Main queues: `eyedar-prod-detection-created`, `eyedar-prod-verify-requested`, `eyedar-prod-verified-animals`

## Repository Layout

```text
infra/                 Terraform root, envs, modules, and IAM policies
services/rust_api/     Rust API service used as the main alert entry point
services/mqtt-monitor/  Optional Rust monitoring service
workers/api/            Legacy Node API service retained for earlier phases
workers/worker-ingest/   SQS ingest worker
workers/worker-verify/   YOLO verification worker
workers/worker-notify/   FCM notification worker
docs/ops/               Deployment and operational documentation
```

## Build and Deploy Flow

The normal path is:

1. Bootstrap AWS access for GitHub Actions with OIDC.
2. Provision the infrastructure with Terraform.
3. Store runtime secrets in AWS Secrets Manager.
4. Build and push container images through GitHub Actions.
5. Deploy the ECS services from the pushed images.

## Prerequisites

- AWS CLI configured for the target account
- Terraform 1.7+ installed
- Docker installed
- Git installed
- A Firebase service account JSON key for notifications

## 1. Bootstrap GitHub Actions Access

Create the GitHub OIDC provider and the deploy role before running Terraform or CI/CD.

The trust policy lives in [infra/iam/github-oidc-trust-policy.json](infra/iam/github-oidc-trust-policy.json) and the permissions policy lives in [infra/iam/github-actions-deployer-policy.json](infra/iam/github-actions-deployer-policy.json).

Replace `YOUR_ACCOUNT_ID` in the trust policy, then run:

```powershell
$AWS_REGION = "us-east-1"

aws iam create-open-id-connect-provider `
  --url https://token.actions.githubusercontent.com `
  --client-id-list sts.amazonaws.com `
  --thumbprint-list 6938fd4d98bab03faadb97b34396831e3780aea1 `
  --region $AWS_REGION

aws iam create-role `
  --role-name eyedar-prod-github-actions-deployer `
  --assume-role-policy-document file://infra/iam/github-oidc-trust-policy.json `
  --region $AWS_REGION

aws iam put-role-policy `
  --role-name eyedar-prod-github-actions-deployer `
  --policy-name eyedar-github-actions-deployer `
  --policy-document file://infra/iam/github-actions-deployer-policy.json `
  --region $AWS_REGION
```

Set the repository variable `AWS_ACCOUNT_ID_A` to your AWS account ID so the workflow can assume the role.

## 2. Provision Infrastructure

The production Terraform root is [infra/envs/prod](infra/envs/prod).

```powershell
cd infra/envs/prod
terraform init
terraform plan
terraform apply
```

After apply, Terraform outputs include the S3 bucket name, queue URLs, ECS cluster name, and GitHub Actions role ARN.

## 3. Configure Secrets

Populate the Secrets Manager values that the services expect:

- DB secret JSON with `username` and `password`
- Firebase service account JSON
- API keys payload for device/app auth

Example:

```powershell
aws secretsmanager put-secret-value `
  --secret-id eyedar-prod-db-password-v3 `
  --secret-string '{"username":"eyedar_admin","password":"<your-db-password>"}' `
  --region us-east-1
```

## 4. Build and Push Images

The preferred build path is GitHub Actions. The workflow in [.github/workflows/deploy.yml](.github/workflows/deploy.yml) builds the service images and pushes them to ECR using the `eyedar-prod-github-actions-deployer` role.

If you want to build manually, use the same image names the workflow uses:

```powershell
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin <account-id>.dkr.ecr.us-east-1.amazonaws.com

docker build -t eyedar-prod-worker-ingest workers/worker-ingest
docker tag eyedar-prod-worker-ingest:latest <account-id>.dkr.ecr.us-east-1.amazonaws.com/eyedar-prod-worker-ingest:latest
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/eyedar-prod-worker-ingest:latest
```

Repeat the same pattern for `worker-verify`, `worker-notify`, `rust_api`, and the API service as needed.

## 5. Run the Stack

Once Terraform, Secrets Manager, and ECR are in place, start the ECS services by forcing a new deployment:

```powershell
aws ecs update-service `
  --cluster eyedar-prod `
  --service eyedar-prod-rust-api `
  --force-new-deployment `
  --region us-east-1

aws ecs update-service `
  --cluster eyedar-prod `
  --service eyedar-prod-worker-ingest `
  --force-new-deployment `
  --region us-east-1
```

The same approach applies to `worker-notify` and `worker-verify` once those services are enabled.

## 6. Validate the System

Use the Rust API test script to confirm the end-to-end alert flow:

```powershell
scripts/test_api_entry_point.ps1
```

The expected path is:

1. Rust API receives the alert.
2. Image is stored in `eyedar-prod-objects-v2`.
3. The detection record is written to PostgreSQL.
4. SQS hands work to `worker-ingest`.

## Infrastructure Notes

- Terraform keeps the S3 bucket name centralized in outputs and environment variables.
- The GitHub OIDC provider and deploy role are required before CI/CD can access AWS.
- The ECS services use private networking, SQS queues, Secrets Manager, and KMS-backed encryption.

## Related Docs

- [docs/ops/DEPLOYMENT.md](docs/ops/DEPLOYMENT.md)
- [infra/README.md](infra/README.md)
- [PROJECT_REVIEW.md](PROJECT_REVIEW.md)
- [terraform_setup.pdf](terraform_setup.pdf)
 
## Quick Links & Notes

- **Run test script:** See [scripts/test_api_entry_point.ps1](scripts/test_api_entry_point.ps1). Run locally with:

```powershell
powershell ./scripts/test_api_entry_point.ps1
```

- **YOLO / `worker-verify`:** The YOLO verification flow is documented in [docs/ops/PROJECT_REVIEW.md](docs/ops/PROJECT_REVIEW.md). The container image for `worker-verify` is built by CI but the service is disabled by default in Terraform. To enable the service, set the variable `worker_verify_desired_count` to a value greater than `0` in the production environment and redeploy (see [infra/envs/prod/variables.tf](infra/envs/prod/variables.tf)). See [workers/worker-verify/README.md](workers/worker-verify/README.md) for runtime notes.

- **Worker READMEs:** Per-worker run and config instructions are in [workers/worker-ingest/README.md](workers/worker-ingest/README.md) and [workers/worker-notify/README.md](workers/worker-notify/README.md).

## Status

The platform is deployed and wired for build/deploy. The remaining work lives in the worker and application feature sets, not in the base infrastructure.
