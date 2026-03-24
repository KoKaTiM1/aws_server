# ACM Module (TLS Certificate)

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "domain_name" {
  description = "Domain name for the TLS certificate"
  type        = string
}

variable "subject_alternative_names" {
  description = "Additional domain names for the certificate"
  type        = list(string)
  default     = []
}

variable "hosted_zone_id" {
  description = "Route53 hosted zone ID for DNS validation"
  type        = string
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
