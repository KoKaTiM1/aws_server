variable "github_deploy_role_arn" {
  description = "ARN of GitHub OIDC role (from github_oidc module)"
  type        = string
}

variable "ecr_repo_arns" {
  description = "List of ECR repository ARNs that GitHub Actions can push to"
  type        = list(string)
}

variable "ecs_cluster_arn" {
  description = "ARN of ECS cluster"
  type        = string
}

variable "ecs_cluster_name" {
  description = "Name of ECS cluster"
  type        = string
}
