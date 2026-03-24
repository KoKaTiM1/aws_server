# ECS Cluster Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "enable_container_insights" {
  description = "Enable CloudWatch Container Insights"
  type        = bool
  default     = false # Can be expensive, enable when needed
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
