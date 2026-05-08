# EyeDAR Infrastructure

Terraform infrastructure for the Eyedar AWS deployment.

## What It Provisions

- VPC, public/private subnets, NAT, VPC endpoints, and security groups
- S3 bucket for detection images
- RDS PostgreSQL for detections and alert records
- ElastiCache Redis for caching and coordination
- SQS queues and DLQs for the detection pipeline
- ECS cluster, task roles, and services
- ECR repositories for the application images
- CloudWatch logs, alarms, and budgets
- GitHub Actions OIDC access for CI/CD

## Layout

```text
infra/
├── envs/
│   └── prod/            # Production environment wiring
├── iam/                 # GitHub OIDC trust and deploy policies
└── modules/             # Reusable Terraform modules
        ├── 00-foundation/   # KMS, Secrets, Tags
        ├── 10-network/      # VPC, NAT, endpoints, security groups
        ├── 20-data/         # S3, RDS, Redis, SQS
        ├── 30-observability/ # CloudWatch, Budgets
        ├── 40-compute/      # ECR, ECS cluster, roles, services
        ├── 50-edge/         # ACM, ALB, WAF
        └── 60-cicd/         # GitHub OIDC integration
```

## Production Stack

The production environment lives in [envs/prod](envs/prod). It wires the module graph and exports the values used by the application services:

- S3 bucket name
- RDS endpoint and database name
- Redis endpoint
- SQS queue URLs and DLQ URLs
- ECS cluster and service names
- GitHub Actions role ARN

## Current Naming

- S3 bucket: `eyedar-prod-objects-v2`
- ECS cluster: `eyedar-prod`
- GitHub deploy role: `eyedar-prod-github-actions-deployer`

## Requirements

- AWS CLI configured for the target account
- Terraform 1.7+ installed
- Permission to create IAM, ECS, RDS, S3, SQS, and CloudWatch resources
- Route 53 hosted zone only if you want TLS/ACM enabled

## Bootstrap the Deploy Role

Before running Terraform from GitHub Actions, create the OIDC provider and deploy role using the policy files in [iam](iam).

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

Set the repository variable `AWS_ACCOUNT_ID_A` to the account ID used in GitHub Actions.

## Build and Apply

```powershell
cd infra/envs/prod
terraform init
terraform plan
terraform apply
```

After apply, use `terraform output` to fetch the S3 bucket name, queue URLs, ECS cluster name, and GitHub Actions role ARN.

## Common Outputs

```powershell
terraform output -json s3_bucket_name
terraform output -json rds_endpoint
terraform output -json sqs_queue_urls
terraform output -json github_actions_role_arn
```

## Notes

- The stack is intentionally modular so services can be rebuilt without reshaping the infrastructure.
- The main application builds and deploys through `.github/workflows/deploy.yml`.
- The infra module tree is the source of truth for AWS resource wiring.
