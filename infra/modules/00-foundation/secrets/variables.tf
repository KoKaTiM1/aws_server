variable "env_name" {
  description = "Environment name (prod, staging, etc)"
  type        = string
}

variable "kms_key_arn" {
  description = "KMS key ARN for secret encryption"
  type        = string
}

variable "db_password" {
  description = "Database password for secret"
  type        = string
  sensitive   = true
  default     = ""
}

variable "db_username" {
  description = "Database username for secret"
  type        = string
  default     = "eyedar_admin"
}

variable "firebase_key" {
  description = "Firebase service account key (JSON string)"
  type        = string
  sensitive   = true
  default     = ""
}

variable "api_keys" {
  description = "API keys configuration (JSON string)"
  type        = string
  sensitive   = true
  default     = ""
}

variable "tags" {
  description = "Common tags applied to all resources"
  type        = map(string)
  default     = {}
}
