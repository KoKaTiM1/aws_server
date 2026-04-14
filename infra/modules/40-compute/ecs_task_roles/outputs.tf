output "task_execution_role_arn" {
  description = "ARN of the ECS task execution role"
  value       = aws_iam_role.ecs_task_execution.arn
}

output "task_role_api_arn" {
  description = "ARN of the API task role"
  value       = aws_iam_role.api.arn
}

output "task_role_rust_api_arn" {
  description = "ARN of the Rust API task role"
  value       = aws_iam_role.rust_api.arn
}

output "task_role_worker_ingest_arn" {
  description = "ARN of the worker-ingest task role"
  value       = aws_iam_role.worker_ingest.arn
}

output "task_role_worker_verify_arn" {
  description = "ARN of the worker-verify task role"
  value       = aws_iam_role.worker_verify.arn
}

output "task_role_worker_notify_arn" {
  description = "ARN of the worker-notify task role"
  value       = aws_iam_role.worker_notify.arn
}
