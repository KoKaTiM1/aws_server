# GitHub OIDC Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "github_org" {
  description = "GitHub organization name"
  type        = string
}

variable "github_repo" {
  description = "GitHub repository name"
  type        = string
}

variable "github_branches" {
  description = "List of branches allowed to assume this role"
  type        = list(string)
  default     = ["main", "prod"]
}

variable "ecr_repository_arns" {
  description = "List of ECR repository ARNs for push access"
  type        = list(string)
}

variable "ecs_cluster_arn" {
  description = "ARN of the ECS cluster"
  type        = string
}

variable "ecs_service_names" {
  description = "List of ECS service names that can be updated"
  type        = list(string)
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
