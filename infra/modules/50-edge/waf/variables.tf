# WAF Module (Web Application Firewall)

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "alb_arn" {
  description = "ARN of the ALB to attach WAF to"
  type        = string
}

variable "enable_rate_limiting" {
  description = "Enable rate limiting rules"
  type        = bool
  default     = true
}

variable "rate_limit" {
  description = "Rate limit (requests per 5 minutes)"
  type        = number
  default     = 2000
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
