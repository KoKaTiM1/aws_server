output "rds_endpoint" {
  description = "RDS instance endpoint"
  value       = aws_db_instance.main.endpoint
}

output "rds_address" {
  description = "RDS instance address (hostname)"
  value       = aws_db_instance.main.address
}

output "rds_port" {
  description = "RDS instance port"
  value       = aws_db_instance.main.port
}

output "rds_db_name" {
  description = "Database name"
  value       = aws_db_instance.main.db_name
}

output "rds_resource_id" {
  description = "RDS resource ID"
  value       = aws_db_instance.main.resource_id
}

output "rds_arn" {
  description = "RDS ARN"
  value       = aws_db_instance.main.arn
}
