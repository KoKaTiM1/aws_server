# Redis Subnet Group
resource "aws_elasticache_subnet_group" "main" {
  name       = "eyedar-${var.env_name}-redis-subnet-group"
  subnet_ids = var.private_subnet_ids

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-redis-subnet-group"
  })
}

# Parameter Group
resource "aws_elasticache_parameter_group" "main" {
  name   = "eyedar-${var.env_name}-redis-params"
  family = "redis7"

  parameter {
    name  = "maxmemory-policy"
    value = "allkeys-lru"
  }

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-redis-params"
  })
}

# Redis Replication Group (supports both single node and multi-node)
resource "aws_elasticache_replication_group" "main" {
  replication_group_id = "eyedar-${var.env_name}-redis"
  description          = "EyeDAR ${var.env_name} Redis cluster"

  engine               = "redis"
  engine_version       = "7.0"
  node_type            = var.node_type
  num_cache_clusters   = var.num_cache_nodes
  parameter_group_name = aws_elasticache_parameter_group.main.name
  port                 = 6379

  subnet_group_name  = aws_elasticache_subnet_group.main.name
  security_group_ids = [var.security_group_id]

  at_rest_encryption_enabled = var.at_rest_encryption_enabled
  transit_encryption_enabled = var.transit_encryption_enabled
  automatic_failover_enabled = var.automatic_failover_enabled && var.num_cache_nodes > 1

  snapshot_retention_limit = 5
  snapshot_window          = "03:00-05:00"
  maintenance_window       = "mon:05:00-mon:07:00"

  auto_minor_version_upgrade = true
  apply_immediately          = false

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-redis"
  })
}
