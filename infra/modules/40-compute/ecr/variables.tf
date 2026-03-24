# ECR Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "image_tag_mutability" {
  description = "Image tag mutability setting"
  type        = string
  default     = "MUTABLE"
}

variable "scan_on_push" {
  description = "Enable image scanning on push"
  type        = bool
  default     = true
}

variable "retention_count" {
  description = "Number of images to keep"
  type        = number
  default     = 10
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
