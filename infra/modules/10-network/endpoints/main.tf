# S3 Gateway Endpoint (high ROI, free data transfer)
resource "aws_vpc_endpoint" "s3" {
  count        = var.enable_s3_endpoint ? 1 : 0
  vpc_id       = var.vpc_id
  service_name = "com.amazonaws.${data.aws_region.current.name}.s3"

  route_table_ids = var.private_route_table_ids

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-s3-endpoint"
  })
}

# Security Group for Interface Endpoints
resource "aws_security_group" "interface_endpoints" {
  count       = var.enable_interface_endpoints ? 1 : 0
  name        = "eyedar-${var.env_name}-interface-endpoints-sg"
  description = "Security group for VPC interface endpoints"
  vpc_id      = var.vpc_id

  ingress {
    description = "HTTPS from VPC"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = [data.aws_vpc.selected.cidr_block]
  }

  egress {
    description = "Allow all outbound"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-interface-endpoints-sg"
  })
}

# CloudWatch Logs Interface Endpoint
resource "aws_vpc_endpoint" "logs" {
  count               = var.enable_interface_endpoints ? 1 : 0
  vpc_id              = var.vpc_id
  service_name        = "com.amazonaws.${data.aws_region.current.name}.logs"
  vpc_endpoint_type   = "Interface"
  subnet_ids          = var.private_subnet_ids
  security_group_ids  = [aws_security_group.interface_endpoints[0].id]
  private_dns_enabled = true

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-logs-endpoint"
  })
}

# ECR API Interface Endpoint
resource "aws_vpc_endpoint" "ecr_api" {
  count               = var.enable_interface_endpoints ? 1 : 0
  vpc_id              = var.vpc_id
  service_name        = "com.amazonaws.${data.aws_region.current.name}.ecr.api"
  vpc_endpoint_type   = "Interface"
  subnet_ids          = var.private_subnet_ids
  security_group_ids  = [aws_security_group.interface_endpoints[0].id]
  private_dns_enabled = true

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-ecr-api-endpoint"
  })
}

# ECR Docker Interface Endpoint
resource "aws_vpc_endpoint" "ecr_dkr" {
  count               = var.enable_interface_endpoints ? 1 : 0
  vpc_id              = var.vpc_id
  service_name        = "com.amazonaws.${data.aws_region.current.name}.ecr.dkr"
  vpc_endpoint_type   = "Interface"
  subnet_ids          = var.private_subnet_ids
  security_group_ids  = [aws_security_group.interface_endpoints[0].id]
  private_dns_enabled = true

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-ecr-dkr-endpoint"
  })
}

# Secrets Manager Interface Endpoint
resource "aws_vpc_endpoint" "secretsmanager" {
  count               = var.enable_interface_endpoints ? 1 : 0
  vpc_id              = var.vpc_id
  service_name        = "com.amazonaws.${data.aws_region.current.name}.secretsmanager"
  vpc_endpoint_type   = "Interface"
  subnet_ids          = var.private_subnet_ids
  security_group_ids  = [aws_security_group.interface_endpoints[0].id]
  private_dns_enabled = true

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-secretsmanager-endpoint"
  })
}

# Data sources
data "aws_region" "current" {}

data "aws_vpc" "selected" {
  id = var.vpc_id
}
