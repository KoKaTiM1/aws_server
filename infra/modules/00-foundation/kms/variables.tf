# KMS Module - Encryption Keys

variable "env_name" {
  description = "Environment name (e.g., prod, staging)"
  type        = string
}

variable "tags" {
  description = "Common tags to apply to all resources"
  type        = map(string)
  default     = {}
}
