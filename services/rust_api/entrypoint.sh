#!/bin/sh

#!/bin/sh
set -e
echo "[entrypoint.sh] Starting entrypoint script..."
echo "[entrypoint.sh] Arguments: $@"
echo "[entrypoint.sh] Environment variables:"
env

# Export AWS credentials from Docker secrets if present
if [ -f /run/secrets/minio_access_key ]; then
  export AWS_ACCESS_KEY_ID="$(cat /run/secrets/minio_access_key)"
  echo "[entrypoint.sh] Exported AWS_ACCESS_KEY_ID from secret."
fi
if [ -f /run/secrets/minio_secret_key ]; then
  export AWS_SECRET_ACCESS_KEY="$(cat /run/secrets/minio_secret_key)"
  echo "[entrypoint.sh] Exported AWS_SECRET_ACCESS_KEY from secret."
fi

# Optionally export DATABASE_URL from a secret if you want (uncomment if needed)
# if [ -f /run/secrets/postgres_password ]; then
#   export DATABASE_URL="postgres://postgres:$(cat /run/secrets/postgres_password)@postgres:5432/eye_dar"
#   echo "[entrypoint.sh] Exported DATABASE_URL from secret."
# fi

ls -l
if [ $# -eq 0 ]; then
    echo "[entrypoint.sh] No arguments provided. Running ./rust_api"
    exec ./rust_api
else
    echo "[entrypoint.sh] Running custom command: $@"
    exec "$@"
fi
    exec "$@"
fi
