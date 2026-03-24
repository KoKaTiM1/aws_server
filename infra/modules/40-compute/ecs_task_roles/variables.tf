# ECS Task Roles Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "s3_bucket_arn" {
  description = "ARN of the S3 objects bucket"
  type        = string
}

variable "sqs_queue_arns" {
  description = "Map of SQS queue ARNs"
  type = object({
    detection_created = string
    verify_requested  = string
    verified_animals  = string
  })
}

variable "secret_arns" {
  description = "Map of Secrets Manager secret ARNs"
  type = object({
    db       = string
    firebase = string
    api_keys = string
  })
}

variable "kms_key_arn" {
  description = "KMS key ARN"
  type        = string
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
