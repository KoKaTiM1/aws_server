# CloudWatch Module (Logs + Alarms)

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "log_retention_days" {
  description = "CloudWatch Logs retention period in days"
  type        = number
  default     = 30
}

variable "kms_key_arn" {
  description = "KMS key ARN for log encryption"
  type        = string
  default     = null
}

variable "alarm_sns_topic_arn" {
  description = "SNS topic ARN for alarm notifications"
  type        = string
  default     = null
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
