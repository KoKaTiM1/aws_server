
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
            "token.actions.githubusercontent.com:sub": [
              "repo:YOUR_ORG/YOUR_REPO:ref:refs/heads/main",
              "repo:YOUR_ORG/YOUR_REPO:ref:refs/heads/*"
            ]
          },
          "StringEquals": {
            "token.actions.githubusercontent.com:aud": "sts.amazonaws.com"
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

# 4. Update trust policy to allow workflow_dispatch (if role already exists)
aws iam update-assume-role-policy \
  --role-name eyedar-prod-github-actions-deployer \
  --policy-document '{
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
            "token.actions.githubusercontent.com:sub": [
              "repo:YOUR_ORG/YOUR_REPO:ref:refs/heads/main",
              "repo:YOUR_ORG/YOUR_REPO:ref:refs/heads/*"
            ]
          },
          "StringEquals": {
            "token.actions.githubusercontent.com:aud": "sts.amazonaws.com"
          }
        }
      }
    ]
  }' \
  --region us-east-1
```

**Note:** Replace `YOUR_ACCOUNT_ID`, `YOUR_ORG`, and `YOUR_REPO` with your actual values.
