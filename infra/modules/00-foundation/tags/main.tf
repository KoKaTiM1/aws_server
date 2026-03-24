locals {
  common_tags = merge({
    Project     = var.project_name
    Environment = var.env_name
    ManagedBy   = "Terraform"
    System      = "eyedar"
  }, var.additional_tags)
}
