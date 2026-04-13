#!/bin/bash
set -e

AWS_ACCOUNT_ID="937115287175"
AWS_REGION="us-east-1"
ECR_REGISTRY="${AWS_ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com"

# Array of services to build
declare -a SERVICES=(
  "api:./workers/api"
  "worker-ingest:./workers/worker-ingest"
  "worker-verify:./workers/worker-verify"
  "worker-notify:./workers/worker-notify"
  "dashboard:./workers/dashboard"
  "rust_api:./services/rust_api"
  "mqtt-monitor:./services/mqtt-monitor"
)

echo "🔨 Starting Docker builds..."
echo "Target registry: $ECR_REGISTRY"
echo ""

# Step 1: Build all images
for SERVICE_DEF in "${SERVICES[@]}"; do
  IFS=':' read -r SERVICE_NAME SERVICE_PATH <<< "$SERVICE_DEF"
  IMAGE_NAME="eyedar-prod-${SERVICE_NAME}"

  echo "📦 Building $SERVICE_NAME..."
  docker build \
    -t "$IMAGE_NAME:latest" \
    -f "$SERVICE_PATH/Dockerfile" \
    "$SERVICE_PATH" || {
    echo "❌ Failed to build $SERVICE_NAME"
    exit 1
  }
  echo "✅ Built $IMAGE_NAME:latest"
  echo ""
done

echo ""
echo "🔑 Authenticating with ECR..."
aws ecr get-login-password --region "$AWS_REGION" | \
  docker login --username AWS --password-stdin "$ECR_REGISTRY" || {
  echo "❌ Failed to authenticate with ECR"
  exit 1
}
echo "✅ Authenticated with ECR"
echo ""

# Step 2: Tag and push all images
echo "🚀 Pushing images to ECR..."
for SERVICE_DEF in "${SERVICES[@]}"; do
  IFS=':' read -r SERVICE_NAME SERVICE_PATH <<< "$SERVICE_DEF"
  IMAGE_NAME="eyedar-prod-${SERVICE_NAME}"
  ECR_URI="${ECR_REGISTRY}/${IMAGE_NAME}:latest"

  echo "📤 Pushing $SERVICE_NAME..."
  docker tag "$IMAGE_NAME:latest" "$ECR_URI"
  docker push "$ECR_URI" || {
    echo "❌ Failed to push $SERVICE_NAME"
    exit 1
  }
  echo "✅ Pushed $ECR_URI"
  echo ""
done

echo "✅ All images built and pushed successfully!"
echo ""
echo "Image URIs:"
for SERVICE_DEF in "${SERVICES[@]}"; do
  IFS=':' read -r SERVICE_NAME _ <<< "$SERVICE_DEF"
  echo "  - ${ECR_REGISTRY}/eyedar-prod-${SERVICE_NAME}:latest"
done
