# NAT Gateway Module

variable "env_name" {
  description = "Environment name"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID"
  type        = string
}

variable "public_subnet_ids" {
  description = "List of public subnet IDs for NAT Gateway placement"
  type        = list(string)
}

variable "private_route_table_ids" {
  description = "List of private route table IDs to update with NAT route"
  type        = list(string)
}

variable "nat_gateway_count" {
  description = "Number of NAT Gateways to create (1 for cost savings, 2+ for HA)"
  type        = number
  default     = 1
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}
