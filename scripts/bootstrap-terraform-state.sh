#!/bin/bash
set -e

# Terraform State Backend Bootstrap Script
# Creates S3 bucket and DynamoDB table for remote state storage
# Run this ONCE before running terraform init

BUCKET_NAME="eyedar-prod-terraform-state"
DYNAMODB_TABLE="eyedar-prod-terraform-locks"
REGION="us-east-1"
AWS_ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)

echo "🚀 Bootstrapping Terraform State Backend"
echo "========================================"
echo "Bucket: $BUCKET_NAME"
echo "DynamoDB Table: $DYNAMODB_TABLE"
echo "Region: $REGION"
echo "Account: $AWS_ACCOUNT_ID"
echo ""

# Step 1: Create S3 bucket
echo "1️⃣  Creating S3 bucket for terraform state..."
if aws s3 ls "s3://$BUCKET_NAME" 2>/dev/null; then
    echo "✅ S3 bucket already exists"
else
    aws s3api create-bucket \
        --bucket "$BUCKET_NAME" \
        --region "$REGION" \
        --create-bucket-configuration LocationConstraint="$REGION" 2>/dev/null || \
    aws s3api create-bucket \
        --bucket "$BUCKET_NAME" \
        --region "$REGION"
    echo "✅ S3 bucket created"
fi

# Step 2: Enable versioning on S3 bucket
echo "2️⃣  Enabling versioning on S3 bucket..."
aws s3api put-bucket-versioning \
    --bucket "$BUCKET_NAME" \
    --versioning-configuration Status=Enabled
echo "✅ Versioning enabled"

# Step 3: Enable encryption on S3 bucket
echo "3️⃣  Enabling AES-256 encryption on S3 bucket..."
aws s3api put-bucket-encryption \
    --bucket "$BUCKET_NAME" \
    --server-side-encryption-configuration '{
        "Rules": [
            {
                "ApplyServerSideEncryptionByDefault": {
                    "SSEAlgorithm": "AES256"
                }
            }
        ]
    }'
echo "✅ Encryption enabled"

# Step 4: Block public access
echo "4️⃣  Blocking all public access to S3 bucket..."
aws s3api put-public-access-block \
    --bucket "$BUCKET_NAME" \
    --public-access-block-configuration \
    "BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true"
echo "✅ Public access blocked"

# Step 5: Create DynamoDB table for state locking
echo "5️⃣  Creating DynamoDB table for terraform locking..."
if aws dynamodb describe-table --table-name "$DYNAMODB_TABLE" --region "$REGION" 2>/dev/null; then
    echo "✅ DynamoDB table already exists"
else
    aws dynamodb create-table \
        --table-name "$DYNAMODB_TABLE" \
        --attribute-definitions AttributeName=LockID,AttributeType=S \
        --key-schema AttributeName=LockID,KeyType=HASH \
        --billing-mode PAY_PER_REQUEST \
        --region "$REGION"

    # Wait for table to be active
    echo "   Waiting for table to be active..."
    aws dynamodb wait table-exists \
        --table-name "$DYNAMODB_TABLE" \
        --region "$REGION"
    echo "✅ DynamoDB table created and active"
fi

echo ""
echo "✅ Terraform state backend bootstrapped successfully!"
echo ""
echo "Next steps:"
echo "1. Uncomment the backend block in infra/envs/prod/providers.tf"
echo "2. Run: terraform init"
echo "3. Confirm state migration to S3"
