variable "env_name" {
  description = "Environment name (prod, staging, etc)"
  type        = string
}

variable "kms_key_arn" {
  description = "KMS key ARN for secret encryption"
  type        = string
}

variable "tags" {
  description = "Common tags applied to all resources"
  type        = map(string)
  default     = {}
}
