output "s3_endpoint_id" {
  description = "ID of the S3 Gateway Endpoint"
  value       = var.enable_s3_endpoint ? aws_vpc_endpoint.s3[0].id : null
}

output "logs_endpoint_id" {
  description = "ID of the CloudWatch Logs Interface Endpoint"
  value       = var.enable_interface_endpoints ? aws_vpc_endpoint.logs[0].id : null
}

output "ecr_api_endpoint_id" {
  description = "ID of the ECR API Interface Endpoint"
  value       = var.enable_interface_endpoints ? aws_vpc_endpoint.ecr_api[0].id : null
}

output "ecr_dkr_endpoint_id" {
  description = "ID of the ECR Docker Interface Endpoint"
  value       = var.enable_interface_endpoints ? aws_vpc_endpoint.ecr_dkr[0].id : null
}

output "secretsmanager_endpoint_id" {
  description = "ID of the Secrets Manager Interface Endpoint"
  value       = var.enable_interface_endpoints ? aws_vpc_endpoint.secretsmanager[0].id : null
}
