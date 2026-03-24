# ElastiCache Redis Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID"
  type        = string
}

variable "private_subnet_ids" {
  description = "List of private subnet IDs for Redis subnet group"
  type        = list(string)
}

variable "security_group_id" {
  description = "Security group ID for Redis"
  type        = string
}

variable "node_type" {
  description = "Redis node type"
  type        = string
  default     = "cache.t4g.micro" # Smallest ARM-based instance for cost savings
}

variable "num_cache_nodes" {
  description = "Number of cache nodes (1 for single node, 2+ for replication)"
  type        = number
  default     = 1
}

variable "automatic_failover_enabled" {
  description = "Enable automatic failover (requires replication group)"
  type        = bool
  default     = false
}

variable "at_rest_encryption_enabled" {
  description = "Enable encryption at rest"
  type        = bool
  default     = true
}

variable "transit_encryption_enabled" {
  description = "Enable encryption in transit (TLS)"
  type        = bool
  default     = false # Simpler for initial setup, enable for production
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
