#!/bin/bash
set -e

echo "🚀 Rust API Starting..."

# Verify required environment variables
: "${DB_HOST:?DB_HOST not set}"
: "${DB_PORT:?DB_PORT not set}"
: "${DB_NAME:?DB_NAME not set}"
: "${DB_PASSWORD:?DB_PASSWORD not set from Secrets Manager}"

# Build DATABASE_URL from components
export DATABASE_URL="postgresql://${DB_USERNAME:-eyedar_admin}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"
echo "✅ DATABASE_URL configured"

exec /app/rust_api
