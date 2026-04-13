output "api_service_name" {
  description = "Name of the API ECS service"
  value       = aws_ecs_service.api.name
}

output "api_service_arn" {
  description = "ARN of the API ECS service"
  value       = aws_ecs_service.api.id
}

output "api_task_definition_arn" {
  description = "ARN of the API task definition"
  value       = aws_ecs_task_definition.api.arn
}

output "api_task_definition_family" {
  description = "Family of the API task definition"
  value       = aws_ecs_task_definition.api.family
}

output "worker_ingest_service_name" {
  description = "Name of the worker-ingest ECS service"
  value       = aws_ecs_service.worker_ingest.name
}

output "worker_ingest_task_definition_arn" {
  description = "ARN of the worker-ingest task definition"
  value       = aws_ecs_task_definition.worker_ingest.arn
}

output "worker_verify_service_name" {
  description = "Name of the worker-verify ECS service (if enabled)"
  value       = var.worker_verify_desired_count > 0 ? aws_ecs_service.worker_verify[0].name : null
}

output "worker_verify_task_definition_arn" {
  description = "ARN of the worker-verify task definition"
  value       = aws_ecs_task_definition.worker_verify.arn
}

output "dashboard_service_name" {
  description = "Name of the dashboard ECS service (if enabled)"
  value       = var.dashboard_desired_count > 0 ? aws_ecs_service.dashboard[0].name : null
}

output "dashboard_task_definition_arn" {
  description = "ARN of the dashboard task definition"
  value       = aws_ecs_task_definition.dashboard.arn
}

output "api_port" {
  description = "Port number for API service"
  value       = 8080
}

output "dashboard_port" {
  description = "Port number for dashboard service"
  value       = 3000
}

output "worker_notify_service_name" {
  description = "Name of the worker-notify ECS service"
  value       = try(aws_ecs_service.worker_notify[0].name, null)
}

output "worker_notify_task_definition_arn" {
  description = "ARN of the worker-notify task definition"
  value       = aws_ecs_task_definition.worker_notify.arn
}

output "rust_api_service_name" {
  description = "Name of the Rust API ECS service (if enabled)"
  value       = var.rust_api_desired_count > 0 ? aws_ecs_service.rust_api[0].name : null
}

output "rust_api_service_arn" {
  description = "ARN of the Rust API ECS service (if enabled)"
  value       = var.rust_api_desired_count > 0 ? aws_ecs_service.rust_api[0].id : null
}

output "rust_api_task_definition_arn" {
  description = "ARN of the Rust API task definition"
  value       = aws_ecs_task_definition.rust_api.arn
}

output "rust_api_port" {
  description = "Port number for Rust API service"
  value       = 8080
}

# Optional services - commented out until ECS services are fully defined
# output "mqtt_monitor_service_name" { ... }
