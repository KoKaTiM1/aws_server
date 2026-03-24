# Network Outputs
output "vpc_id" {
  description = "VPC ID"
  value       = module.vpc.vpc_id
}

output "nat_gateway_ips" {
  description = "NAT Gateway public IP addresses"
  value       = module.nat.nat_eip_addresses
}

# Data Outputs
output "s3_bucket_name" {
  description = "S3 bucket name for objects"
  value       = module.s3_objects.s3_bucket_name
}

output "rds_endpoint" {
  description = "RDS endpoint"
  value       = module.rds.rds_endpoint
  sensitive   = true
}

output "rds_db_name" {
  description = "RDS database name"
  value       = module.rds.rds_db_name
}

output "redis_endpoint" {
  description = "Redis endpoint"
  value       = module.redis.redis_endpoint
  sensitive   = true
}

output "sqs_queue_urls" {
  description = "SQS queue URLs"
  value = {
    detection_created = module.sqs.queue_url_detection_created
    verify_requested  = module.sqs.queue_url_verify_requested
  }
}

# Compute Outputs
output "ecr_repository_urls" {
  description = "ECR repository URLs"
  value = {
    api            = module.ecr.ecr_repo_url_api
    worker_ingest  = module.ecr.ecr_repo_url_worker_ingest
    worker_verify  = module.ecr.ecr_repo_url_worker_verify
    dashboard      = module.ecr.ecr_repo_url_dashboard
  }
}

output "ecs_cluster_name" {
  description = "ECS cluster name"
  value       = module.ecs_cluster.ecs_cluster_name
}

output "ecs_service_names" {
  description = "ECS service names"
  value = {
    api            = module.ecs_services.api_service_name
    worker_ingest  = module.ecs_services.worker_ingest_service_name
    worker_verify  = module.ecs_services.worker_verify_service_name
    dashboard      = module.ecs_services.dashboard_service_name
  }
}

# Edge Outputs
output "alb_dns_name" {
  description = "ALB DNS name"
  value       = module.alb.alb_dns_name
}

# ACM certificate output - disabled for HTTP-only mode
# output "acm_certificate_arn" {
#   description = "ACM certificate ARN"
#   value       = module.acm.acm_certificate_arn
# }

# CI/CD Outputs
output "github_actions_role_arn" {
  description = "IAM role ARN for GitHub Actions"
  value       = module.github_oidc.github_actions_role_arn
}

# Observability Outputs
output "cloudwatch_dashboard_name" {
  description = "CloudWatch dashboard name"
  value       = module.cloudwatch.dashboard_name
}

output "cloudwatch_log_groups" {
  description = "CloudWatch log group names"
  value = {
    api            = module.cloudwatch.log_group_api_name
    worker_ingest  = module.cloudwatch.log_group_worker_ingest_name
    worker_verify  = module.cloudwatch.log_group_worker_verify_name
    dashboard      = module.cloudwatch.log_group_dashboard_name
  }
}

# Secret ARNs (for manual updates)
output "secret_arns" {
  description = "Secrets Manager secret ARNs"
  value = {
    db       = module.secrets.db_secret_arn
    firebase = module.secrets.firebase_secret_arn
    api_keys = module.secrets.api_keys_secret_arn
  }
}
