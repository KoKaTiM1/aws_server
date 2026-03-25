# Environment
variable "env_name" {
  description = "Environment name"
  type        = string
  default     = "prod"
}

variable "region" {
  description = "AWS region"
  type        = string
  default     = "us-east-1"
}

# Network Configuration
variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}

variable "availability_zones" {
  description = "List of availability zones"
  type        = list(string)
  default     = ["us-east-1a", "us-east-1b"]
}

variable "public_subnet_cidrs" {
  description = "CIDR blocks for public subnets"
  type        = list(string)
  default     = ["10.0.1.0/24", "10.0.2.0/24"]
}

variable "private_subnet_cidrs" {
  description = "CIDR blocks for private subnets"
  type        = list(string)
  default     = ["10.0.11.0/24", "10.0.12.0/24"]
}

variable "nat_gateway_count" {
  description = "Number of NAT Gateways (1 for cost savings, 2 for HA)"
  type        = number
  default     = 1
}

variable "enable_s3_endpoint" {
  description = "Enable S3 VPC endpoint (recommended for cost savings)"
  type        = bool
  default     = true
}

variable "enable_interface_endpoints" {
  description = "Enable interface endpoints (logs, ecr, secrets) - can be expensive"
  type        = bool
  default     = false
}

# Domain Configuration
variable "domain_name" {
  description = "Domain name for TLS certificate"
  type        = string
}

variable "hosted_zone_id" {
  description = "Route53 hosted zone ID"
  type        = string
}

# Database Configuration
variable "rds_instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t4g.micro"
}

variable "rds_allocated_storage" {
  description = "RDS allocated storage in GB"
  type        = number
  default     = 20
}

variable "rds_multi_az" {
  description = "Enable RDS Multi-AZ"
  type        = bool
  default     = false
}

# Redis Configuration
variable "redis_node_type" {
  description = "Redis node type"
  type        = string
  default     = "cache.t4g.micro"
}

variable "redis_num_cache_nodes" {
  description = "Number of Redis cache nodes"
  type        = number
  default     = 1
}

# ECS Configuration
variable "image_tag_api" {
  description = "Docker image tag for API service - CI/CD should override with git SHA"
  type        = string
  default     = "latest"
}

variable "image_tag_worker_ingest" {
  description = "Docker image tag for worker-ingest service - CI/CD should override with git SHA"
  type        = string
  default     = "latest"
}

variable "image_tag_worker_verify" {
  description = "Docker image tag for worker-verify service - CI/CD should override with git SHA"
  type        = string
  default     = "latest"
}

variable "image_tag_worker_notify" {
  description = "Docker image tag for worker-notify service - CI/CD should override with git SHA"
  type        = string
  default     = "latest"
}

variable "image_tag_dashboard" {
  description = "Docker image tag for dashboard service - CI/CD should override with git SHA"
  type        = string
  default     = "latest"
}

variable "api_desired_count" {
  description = "Desired count of API tasks"
  type        = number
  default     = 1
}

variable "worker_ingest_desired_count" {
  description = "Desired count of ingest worker tasks"
  type        = number
  default     = 1
}

variable "worker_verify_desired_count" {
  description = "Desired count of verify worker tasks (0 to disable)"
  type        = number
  default     = 0
}

variable "worker_notify_desired_count" {
  description = "Desired count of notify worker tasks (0 to disable)"
  type        = number
  default     = 1
}

variable "dashboard_desired_count" {
  description = "Desired count of dashboard tasks (0 to disable)"
  type        = number
  default     = 1
}

# Observability Configuration
variable "log_retention_days" {
  description = "CloudWatch Logs retention period in days"
  type        = number
  default     = 30
}

variable "enable_container_insights" {
  description = "Enable CloudWatch Container Insights"
  type        = bool
  default     = false
}

variable "monthly_budget_amount" {
  description = "Monthly budget limit in USD"
  type        = number
  default     = 100  # Start lean, upgrade later as needed
}

variable "budget_alert_emails" {
  description = "Email addresses for budget alerts"
  type        = list(string)
}

# WAF Configuration
variable "enable_waf" {
  description = "Enable WAF for ALB"
  type        = bool
  default     = true
}

variable "waf_rate_limit" {
  description = "WAF rate limit (requests per 5 minutes)"
  type        = number
  default     = 2000
}

# CI/CD Configuration
variable "github_org" {
  description = "GitHub organization name"
  type        = string
}

variable "github_repo" {
  description = "GitHub repository name"
  type        = string
}

variable "github_branches" {
  description = "GitHub branches allowed to deploy"
  type        = list(string)
  default     = ["main", "prod"]
}
