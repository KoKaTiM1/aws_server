output "log_group_api_name" {
  description = "Name of the API log group"
  value       = aws_cloudwatch_log_group.api.name
}

output "log_group_api_arn" {
  description = "ARN of the API log group"
  value       = aws_cloudwatch_log_group.api.arn
}

output "log_group_worker_ingest_name" {
  description = "Name of the worker-ingest log group"
  value       = aws_cloudwatch_log_group.worker_ingest.name
}

output "log_group_worker_ingest_arn" {
  description = "ARN of the worker-ingest log group"
  value       = aws_cloudwatch_log_group.worker_ingest.arn
}

output "log_group_worker_verify_name" {
  description = "Name of the worker-verify log group"
  value       = aws_cloudwatch_log_group.worker_verify.name
}

output "log_group_worker_verify_arn" {
  description = "ARN of the worker-verify log group"
  value       = aws_cloudwatch_log_group.worker_verify.arn
}

output "log_group_dashboard_name" {
  description = "Name of the dashboard log group"
  value       = aws_cloudwatch_log_group.dashboard.name
}

output "log_group_dashboard_arn" {
description = "ARN of the dashboard log group"
  value       = aws_cloudwatch_log_group.dashboard.arn
}

output "log_group_worker_notify_name" {
  description = "Name of the worker-notify log group"
  value       = aws_cloudwatch_log_group.worker_notify.name
}

output "log_group_worker_notify_arn" {
  description = "ARN of the worker-notify log group"
  value       = aws_cloudwatch_log_group.worker_notify.arn
}

output "log_group_rust_api_name" {
  description = "Name of the Rust API log group"
  value       = aws_cloudwatch_log_group.rust_api.name
}

output "log_group_rust_api_arn" {
  description = "ARN of the Rust API log group"
  value       = aws_cloudwatch_log_group.rust_api.arn
}

output "log_group_scheduler_name" {
  description = "Name of the scheduler log group"
  value       = aws_cloudwatch_log_group.scheduler.name
}

output "log_group_scheduler_arn" {
  description = "ARN of the scheduler log group"
  value       = aws_cloudwatch_log_group.scheduler.arn
}

output "log_group_alb_name" {
  description = "Name of the ALB log group"
  value       = aws_cloudwatch_log_group.alb.name
}

output "alarm_sns_topic_arn" {
  description = "ARN of the SNS topic for alarms"
  value       = var.alarm_sns_topic_arn != null ? var.alarm_sns_topic_arn : aws_sns_topic.alarms[0].arn
}

output "dashboard_name" {
  description = "Name of the CloudWatch dashboard"
  value       = aws_cloudwatch_dashboard.main.dashboard_name
}

# Optional service log groups - commented out until services are fully defined
# output "log_group_rust_api_name" { ... }
# output "log_group_mqtt_monitor_name" { ... }
