output "db_secret_arn" {
  description = "ARN of the database password secret"
  value       = aws_secretsmanager_secret.db.arn
}

output "firebase_secret_arn" {
  description = "ARN of the Firebase key secret"
  value       = aws_secretsmanager_secret.firebase.arn
}

output "api_keys_secret_arn" {
  description = "ARN of the API keys secret"
  value       = aws_secretsmanager_secret.api_keys.arn
}
