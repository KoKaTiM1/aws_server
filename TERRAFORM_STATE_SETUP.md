# Terraform State Backend Setup

This document explains how to set up the terraform state backend for GitHub Actions-based infrastructure management.

## ✅ What's Been Set Up

1. **Bootstrap Script**: `scripts/bootstrap-terraform-state.sh`
   - Creates S3 bucket for terraform state
   - Creates DynamoDB table for state locking
   - Enables encryption and versioning

2. **Terraform Backend**: `infra/envs/prod/providers.tf`
   - Configured to use S3 remote backend
   - DynamoDB locking prevents concurrent applies

3. **GitHub Actions Workflow**: `.github/workflows/terraform.yml`
   - Runs `terraform plan` on PRs
   - Runs `terraform apply` when code is merged to main
   - Can manually trigger `destroy` via workflow_dispatch

## 🚀 Setup Instructions

### Step 1: Run Bootstrap Script (One Time)

```bash
# Make script executable
chmod +x scripts/bootstrap-terraform-state.sh

# Run bootstrap (creates S3 bucket + DynamoDB table)
./scripts/bootstrap-terraform-state.sh
```

This creates:
- S3 bucket: `eyedar-prod-terraform-state`
- DynamoDB table: `eyedar-prod-terraform-locks`

### Step 2: Initialize Terraform

Once bootstrap is complete:

```bash
cd infra/envs/prod

# Initialize terraform with backend
terraform init

# Confirm to copy existing state to S3 (if you have local state)
# Type: yes
```

### Step 3: Verify Backend Configuration

```bash
# Check if backend is configured
terraform state list
```

If it returns resources (instead of an error), the backend is working ✅

## 🔄 GitHub Actions Workflow

### Automatic Triggers

1. **On PR to main** (creates plan comment)
   ```
   git checkout -b feature-branch
   # Make changes to .tf files
   git push
   # GitHub Actions runs terraform plan
   # Results appear as PR comment
   ```

2. **On merge to main** (applies changes)
   ```
   # After PR approval and merge to main
   # GitHub Actions automatically runs terraform apply
   ```

### Manual Triggers

You can manually trigger destroy (or apply):

```bash
# Go to GitHub repo
# Actions → Terraform Plan & Apply → Run workflow
# Select input: destroy
# Run workflow
```

## 📊 State Storage Architecture

```
GitHub Actions Runner (ephemeral)
          ↓
   terraform init/plan/apply
          ↓
   Read/Write State: S3 + DynamoDB
          ↓
Locking: DynamoDB (prevents conflicts)
```

**Why S3 + DynamoDB?**
- State persists across GitHub Actions runs
- DynamoDB locking prevents team conflicts
- Encrypted at rest
- Versioned for rollback capability

## ⚠️ Important Notes

1. **Never commit `terraform.tfstate`** to git
   - It's stored in S3, not in this repo
   - `.gitignore` already excludes it

2. **GitHub OIDC Role** must have permissions for:
   - S3 bucket operations
   - DynamoDB operations
   - All infrastructure resources

3. **State is sensitive** - contains:
   - RDS passwords
   - Database connection strings
   - AWS resource IDs
   - Access keys (encrypted)

4. **Always run `terraform plan` first** to review changes before applying

## 🔐 Security

- S3 bucket: encrypted with AES-256
- Public access: blocked
- Versioning: enabled for recovery
- DynamoDB: point-in-time recovery capable

## 📝 Workflow

```
Local Development:
  1. Create feature branch
  2. Modify .tf files
  3. Push to GitHub
  4. GitHub Actions runs terraform plan
  5. Review plan in PR comment
  
Deployment:
  1. Approve PR
  2. Merge to main
  3. GitHub Actions runs terraform apply
  4. Infrastructure is updated in AWS
  5. State is saved to S3

Troubleshooting:
  1. Check GitHub Actions logs for errors
  2. Verify AWS credentials are correct
  3. Confirm S3 bucket and DynamoDB table exist
  4. Check IAM role permissions
```

## 🆘 Troubleshooting

### "Backend initialization failed"
```bash
# Verify S3 bucket exists
aws s3 ls s3://eyedar-prod-terraform-state

# Verify DynamoDB table exists
aws dynamodb describe-table --table-name eyedar-prod-terraform-locks
```

### "State lock timeout"
- Another terraform operation is in progress
- Wait 10 minutes or manually unlock:
```bash
# Only do this if process has crashed
terraform force-unlock [LOCK_ID]
```

### "Permission denied" errors
- Check GitHub OIDC role has required permissions
- Verify role ARN in `.github/workflows/terraform.yml`

## 📚 Additional Resources

- [Terraform Remote State](https://www.terraform.io/language/state/remote)
- [S3 Backend Configuration](https://www.terraform.io/language/settings/backends/s3)
- [GitHub OIDC Provider](https://docs.github.com/en/actions/deployment/security-hardening-your-deployments/about-security-hardening-with-openid-connect)
