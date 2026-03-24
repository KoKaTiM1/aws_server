# VPC Endpoints Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID"
  type        = string
}

variable "private_subnet_ids" {
  description = "List of private subnet IDs"
  type        = list(string)
}

variable "private_route_table_ids" {
  description = "List of private route table IDs"
  type        = list(string)
}

variable "enable_s3_endpoint" {
  description = "Enable S3 Gateway Endpoint"
  type        = bool
  default     = true
}

variable "enable_interface_endpoints" {
  description = "Enable interface endpoints (logs, ecr, secrets)"
  type        = bool
  default     = false # Can be expensive, enable when budget allows
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
