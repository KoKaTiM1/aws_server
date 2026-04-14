terraform {
  required_version = ">= 1.5.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }

  # Backend configuration for state storage
  # Stores terraform state in S3 with DynamoDB locking for team collaboration
  backend "s3" {
    bucket         = "eyedar-prod-terraform-state"
    key            = "prod/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "eyedar-prod-terraform-locks"
  }
}

provider "aws" {
  region = var.region

  default_tags {
    tags = {
      Environment = var.env_name
      Project     = "eyedar"
      ManagedBy   = "Terraform"
    }
  }
}
