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

# Handle DB_PASSWORD - it might be a JSON object from Secrets Manager
# If it looks like JSON (starts with '{'), extract the password field
if [[ "$DB_PASSWORD" == "{"* ]]; then
    echo "🔍 DB_PASSWORD appears to be JSON - extracting password field..."
    # Use jq to extract password, or fall back to grep if jq is not available
    if command -v jq &> /dev/null; then
        EXTRACTED_PASSWORD=$(echo "$DB_PASSWORD" | jq -r '.password')
        if [ -z "$EXTRACTED_PASSWORD" ] || [ "$EXTRACTED_PASSWORD" = "null" ]; then
            echo "❌ Failed to extract password from JSON"
            exit 1
        fi
        DB_PASSWORD="$EXTRACTED_PASSWORD"
        echo "✅ Extracted password from JSON"
    else
        # Fallback: use grep to extract the password field
        EXTRACTED_PASSWORD=$(echo "$DB_PASSWORD" | grep -o '"password":"[^"]*"' | cut -d'"' -f4)
        if [ -z "$EXTRACTED_PASSWORD" ]; then
            echo "❌ Failed to extract password from JSON (jq not available)"
            exit 1
        fi
        DB_PASSWORD="$EXTRACTED_PASSWORD"
        echo "✅ Extracted password from JSON using grep"
    fi
fi

echo "DB_PASSWORD=${DB_PASSWORD:0:10}***"

# URL-encode the password to handle special characters
# Simple bash function to URL-encode strings
urlencode() {
    local string="$1"
    local strlen=${#string}
    local encoded=""
    local pos c o

    for (( pos=0 ; pos<strlen ; pos++ )); do
        c=${string:$pos:1}
        case "$c" in
            [-_.~a-zA-Z0-9] ) o="${c}" ;;
            * ) printf -v o '%%%02x' "'$c"
        esac
        encoded+="${o}"
    done
    echo "${encoded}"
}

ENCODED_PASSWORD=$(urlencode "$DB_PASSWORD")

# Build DATABASE_URL from components
export DATABASE_URL="postgresql://${DB_USERNAME:-eyedar_admin}:${ENCODED_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"
echo "✅ DATABASE_URL configured: postgresql://${DB_USERNAME:-eyedar_admin}:***@${DB_HOST}:${DB_PORT}/${DB_NAME}"
echo "DATABASE_URL=$DATABASE_URL"

echo ""
echo "🔄 Running database migrations..."

# Run migrations using sqlx-cli if available, otherwise use psql
if command -v sqlx &> /dev/null; then
    echo "📦 Using sqlx-cli for migrations..."
    # Run sqlx migrations from /migrations directory
    sqlx migrate run --database-url "$DATABASE_URL" --source /app/migrations 2>&1
    if [ $? -eq 0 ]; then
        echo "✅ Migrations completed successfully"
    else
        echo "⚠️ Migrations failed or already applied - continuing..."
    fi
elif command -v psql &> /dev/null; then
    echo "📦 Using psql for migrations..."
    export PGPASSWORD="$DB_PASSWORD"
    psql -h "$DB_HOST" -p "$DB_PORT" -U "${DB_USERNAME:-eyedar_admin}" -d "$DB_NAME" \
        -f /app/migrations/001_init_schema.sql \
        -f /app/migrations/002_rust_api_tables.sql 2>&1
    if [ $? -eq 0 ]; then
        echo "✅ Migrations completed successfully"
    else
        echo "⚠️ Migrations may have failed - continuing..."
    fi
else
    echo "⚠️ Neither sqlx nor psql available - skipping migrations"
fi

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
