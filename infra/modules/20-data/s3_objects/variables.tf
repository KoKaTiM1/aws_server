# S3 Objects Storage Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "kms_key_arn" {
  description = "KMS key ARN for bucket encryption"
  type        = string
}

variable "force_destroy" {
  description = "Allow Terraform to destroy bucket even if it contains objects"
  type        = bool
  default     = false
}

variable "lifecycle_enabled" {
  description = "Enable lifecycle policies for cost optimization"
  type        = bool
  default     = true
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
