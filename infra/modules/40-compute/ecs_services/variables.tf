# ECS Services Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "ecs_cluster_arn" {
  description = "ARN of the ECS cluster"
  type        = string
}

variable "private_subnet_ids" {
  description = "List of private subnet IDs for ECS tasks"
  type        = list(string)
}

variable "security_group_ids" {
  description = "Map of security group IDs for each service"
  type = object({
    api     = string
    workers = string
  })
}

variable "task_execution_role_arn" {
  description = "ARN of the task execution role"
  type        = string
}

variable "task_role_arns" {
  description = "Map of task role ARNs"
  type = object({
    api           = string
    rust_api      = string
    worker_ingest = string
    worker_verify = string
    worker_notify = string
  })
}

variable "log_group_names" {
  description = "Map of CloudWatch log group names"
  type = object({
    api           = string
    rust_api      = string
    worker_ingest = string
    worker_verify = string
    worker_notify = string
  })
}

variable "ecr_image_urls" {
  description = "Map of ECR image URLs"
  type = object({
    api           = string
    rust_api      = string
    worker_ingest = string
    worker_verify = string
    worker_notify = string
  })
}

variable "image_tags" {
  description = "Map of image tags for each service"
  type = object({
    api           = string
    rust_api      = string
    worker_ingest = string
    worker_verify = string
    worker_notify = string
  })
  default = {
    api           = "latest"
    rust_api      = "latest"
    worker_ingest = "latest"
    worker_verify = "latest"
    worker_notify = "latest"
  }
}

variable "environment_vars" {
  description = "Environment variables for services"
  type = object({
    rds_host                       = string
    rds_port                       = string
    rds_db_name                    = string
    rds_user                       = string
    rds_password                   = string
    redis_host                     = string
    redis_port                     = string
    s3_bucket_name                 = string
    sqs_queue_url_detection        = string
    sqs_queue_url_verify           = string
    sqs_queue_url_verified_animals = string
  })
}

variable "secret_arns" {
  description = "Map of secret ARNs for secrets injection"
  type = object({
    db       = string
    firebase = string
    api_keys = string
  })
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
  description = "Desired count of verify worker tasks"
  type        = number
  default     = 0 # Optional service
}

variable "worker_notify_desired_count" {
  description = "Desired count of notify worker tasks"
  type        = number
  default     = 1
}

variable "rust_api_desired_count" {
  description = "Desired count of Rust API tasks"
  type        = number
  default     = 1
}

variable "enable_autoscaling" {
  description = "Enable autoscaling for services"
  type        = bool
  default     = false
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}

variable "alb_target_group_arn_api" {
  description = "ARN of the ALB target group for API services"
  type        = string
}
