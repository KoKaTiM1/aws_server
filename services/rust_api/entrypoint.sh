#!/bin/bash
set -x  # Enable debug output
# set -e  # REMOVED: Don't exit on error so we can debug

echo "🚀 Rust API Starting..."
echo "Current user: $(whoami)"
echo "Current directory: $(pwd)"
echo "Available env vars:"
env | grep -E "DB_|AWS_|RUST_|PORT|S3_" | head -20

echo ""
echo "Checking required environment variables..."

# Verify required environment variables
if [ -z "$DB_HOST" ]; then echo "❌ DB_HOST not set"; exit 1; fi
if [ -z "$DB_PORT" ]; then echo "❌ DB_PORT not set"; exit 1; fi
if [ -z "$DB_NAME" ]; then echo "❌ DB_NAME not set"; exit 1; fi
if [ -z "$DB_PASSWORD" ]; then echo "❌ DB_PASSWORD not set from Secrets Manager"; exit 1; fi

echo "✅ All required env vars present"
echo "DB_HOST=$DB_HOST"
echo "DB_PORT=$DB_PORT"
echo "DB_NAME=$DB_NAME"
echo "DB_PASSWORD=${DB_PASSWORD:0:10}***"

# Build DATABASE_URL from components
export DATABASE_URL="postgresql://${DB_USERNAME:-eyedar_admin}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"
echo "✅ DATABASE_URL configured: postgresql://${DB_USERNAME:-eyedar_admin}:***@${DB_HOST}:${DB_PORT}/${DB_NAME}"
echo "DATABASE_URL=$DATABASE_URL"

echo ""
echo "Starting Rust API binary..."
echo "Binary path: /app/rust_api"
ls -la /app/rust_api

echo ""
echo "Executing binary..."
exec /app/rust_api
exit_code=$?
echo "Binary exited with code: $exit_code"
exit $exit_code
