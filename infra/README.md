# EyeDAR Infrastructure - AWS ECS Terraform

This repository contains the complete ECS-based AWS infrastructure for the EyeDAR system.

## Architecture Overview

- **Compute**: ECS Fargate (API + Workers + Dashboard)
- **Data**: RDS PostgreSQL + ElastiCache Redis + S3
- **Messaging**: SQS with Dead Letter Queues
- **Edge**: ALB with WAF protection
- **Networking**: VPC with public/private subnets, NAT Gateway, VPC Endpoints

## Structure

```
infra/
├── envs/
│   └── prod/          # Production environment configuration
└── modules/           # Reusable Terraform modules
    ├── 00-foundation/ # IAM, KMS, Secrets
    ├── 10-network/    # VPC, Subnets, NAT, Endpoints, Security Groups
    ├── 20-data/       # S3, RDS, Redis, SQS
    ├── 30-observability/ # CloudWatch, Budgets
    ├── 40-compute/    # ECR, ECS Cluster, Task Roles, Services
    ├── 50-edge/       # ACM, ALB, WAF
    └── 60-cicd/       # GitHub Actions OIDC
```

## Deployment Order

1. Foundation (KMS, Secrets)
2. Network (VPC → NAT → Endpoints → Security Groups)
3. Data (S3 → RDS → Redis → SQS)
4. Observability (CloudWatch, Budgets)
5. Compute (ECR → ECS Cluster → Task Roles → Services)
6. Edge (ACM → ALB → WAF)
7. CI/CD (GitHub OIDC)

## Prerequisites

- AWS CLI configured
- Terraform >= 1.5.0
- AWS account with appropriate permissions
- Domain name and hosted zone in Route 53 (for TLS)

## Usage

```bash
cd infra/envs/prod
terraform init
terraform plan
terraform apply
```

## Cost Optimization

- Single NAT Gateway (upgrade to 2 for HA)
- S3 VPC Gateway Endpoint (no data transfer costs)
- Fargate Spot (optional for non-critical workloads)
- RDS small instance (scale up as needed)
- Redis small instance (scale up as needed)
