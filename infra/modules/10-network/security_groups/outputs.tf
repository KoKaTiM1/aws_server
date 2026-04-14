output "sg_alb_public_id" {
  description = "Security group ID for public ALB"
  value       = aws_security_group.alb_public.id
}

output "sg_ecs_api_id" {
  description = "Security group ID for ECS API service"
  value       = aws_security_group.ecs_api.id
}

output "sg_ecs_workers_id" {
  description = "Security group ID for ECS worker services"
  value       = aws_security_group.ecs_workers.id
}

output "sg_rds_id" {
  description = "Security group ID for RDS database"
  value       = aws_security_group.rds.id
}

output "sg_redis_id" {
  description = "Security group ID for ElastiCache Redis"
  value       = aws_security_group.redis.id
}
