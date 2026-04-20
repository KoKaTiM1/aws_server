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

resource "aws_secretsmanager_secret_version" "db" {
  count             = var.db_password != "" ? 1 : 0
  secret_id         = aws_secretsmanager_secret.db.id
  secret_string     = jsonencode({
    username = var.db_username
    password = var.db_password
  })
  depends_on        = [aws_secretsmanager_secret.db]
}

resource "aws_secretsmanager_secret" "firebase" {
  name                    = "eyedar-${var.env_name}-firebase-key-v3"
  recovery_window_in_days = 7
  kms_key_id              = var.kms_key_arn

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-firebase-key-v3"
  })
}

resource "aws_secretsmanager_secret_version" "firebase" {
  count             = var.firebase_key != "" ? 1 : 0
  secret_id         = aws_secretsmanager_secret.firebase.id
  secret_string     = var.firebase_key
  depends_on        = [aws_secretsmanager_secret.firebase]
}

resource "aws_secretsmanager_secret" "api_keys" {
  name                    = "eyedar-${var.env_name}-api-keys-v3"
  recovery_window_in_days = 7
  kms_key_id              = var.kms_key_arn

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-api-keys-v3"
  })
}

resource "aws_secretsmanager_secret_version" "api_keys" {
  count             = var.api_keys != "" ? 1 : 0
  secret_id         = aws_secretsmanager_secret.api_keys.id
  secret_string     = var.api_keys
  depends_on        = [aws_secretsmanager_secret.api_keys]
}
