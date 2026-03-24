# Common Tags Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "project_name" {
  description = "Project name"
  type        = string
  default     = "eyedar"
}

variable "additional_tags" {
  description = "Additional tags to merge"
  type        = map(string)
  default     = {}
}
