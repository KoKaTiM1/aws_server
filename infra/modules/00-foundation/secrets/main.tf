# Secrets Manager - stores DB password, Firebase key, API keys
# Values are NOT stored here - they are created/updated in Secrets Manager console
# Apps fetch these at runtime using IAM role permissions

resource "aws_secretsmanager_secret" "db" {
  name                    = "eyedar-${var.env_name}-db-password-v3"
  recovery_window_in_days = 7
  kms_key_id              = var.kms_key_arn

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-db-password-v3"
  })
}

resource "aws_secretsmanager_secret" "firebase" {
  name                    = "eyedar-${var.env_name}-firebase-key-v3"
  recovery_window_in_days = 7
  kms_key_id              = var.kms_key_arn

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-firebase-key-v3"
  })
}

resource "aws_secretsmanager_secret" "api_keys" {
  name                    = "eyedar-${var.env_name}-api-keys-v3"
  recovery_window_in_days = 7
  kms_key_id              = var.kms_key_arn

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-api-keys-v3"
  })
}
