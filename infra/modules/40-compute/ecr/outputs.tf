output "ecr_repo_url_api" {
  description = "URL of the API ECR repository"
  value       = aws_ecr_repository.api.repository_url
}

output "ecr_repo_arn_api" {
  description = "ARN of the API ECR repository"
  value       = aws_ecr_repository.api.arn
}

output "ecr_repo_url_worker_ingest" {
  description = "URL of the worker-ingest ECR repository"
  value       = aws_ecr_repository.worker_ingest.repository_url
}

output "ecr_repo_arn_worker_ingest" {
  description = "ARN of the worker-ingest ECR repository"
  value       = aws_ecr_repository.worker_ingest.arn
}

output "ecr_repo_url_worker_verify" {
  description = "URL of the worker-verify ECR repository"
  value       = aws_ecr_repository.worker_verify.repository_url
}

output "ecr_repo_arn_worker_verify" {
  description = "ARN of the worker-verify ECR repository"
  value       = aws_ecr_repository.worker_verify.arn
}

output "ecr_repo_url_dashboard" {
  description = "URL of the dashboard ECR repository"
  value       = aws_ecr_repository.dashboard.repository_url
}

output "ecr_repo_arn_dashboard" {
  description = "ARN of the dashboard ECR repository"
  value       = aws_ecr_repository.dashboard.arn
}

output "ecr_repo_url_worker_notify" {
  description = "URL of the worker-notify ECR repository"
  value       = aws_ecr_repository.worker_notify.repository_url
}

output "ecr_repo_arn_worker_notify" {
  description = "ARN of the worker-notify ECR repository"
  value       = aws_ecr_repository.worker_notify.arn
}

output "ecr_repo_url_rust_api" {
  description = "URL of the rust-api ECR repository"
  value       = aws_ecr_repository.rust_api.repository_url
}

output "ecr_repo_arn_rust_api" {
  description = "ARN of the rust-api ECR repository"
  value       = aws_ecr_repository.rust_api.arn
}

output "ecr_repo_url_mqtt_monitor" {
  description = "URL of the mqtt-monitor ECR repository"
  value       = aws_ecr_repository.mqtt_monitor.repository_url
}

output "ecr_repo_arn_mqtt_monitor" {
  description = "ARN of the mqtt-monitor ECR repository"
  value       = aws_ecr_repository.mqtt_monitor.arn
}
