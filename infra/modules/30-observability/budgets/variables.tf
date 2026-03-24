# AWS Budgets Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "monthly_budget_amount" {
  description = "Monthly budget limit in USD"
  type        = number
  default     = 100
}

variable "alert_threshold_percentages" {
  description = "Budget threshold percentages for alerts"
  type        = list(number)
  default     = [80, 100]
}

variable "alert_emails" {
  description = "Email addresses to receive budget alerts"
  type        = list(string)
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
