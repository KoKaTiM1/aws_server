# SQS Queues Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "kms_key_id" {
  description = "KMS key ID for queue encryption"
  type        = string
  default     = null
}

variable "message_retention_seconds" {
  description = "Message retention period in seconds"
  type        = number
  default     = 345600 # 4 days
}

variable "visibility_timeout_seconds" {
  description = "Visibility timeout for messages"
  type        = number
  default     = 30
}

variable "max_receive_count" {
  description = "Maximum number of receives before sending to DLQ"
  type        = number
  default     = 3
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
