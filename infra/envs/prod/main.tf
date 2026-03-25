# Production Environment Configuration
# This file wires all modules together according to the planned architecture

locals {
  env_name = var.env_name

  # Common tags for all resources
  tags = module.tags.tags
}

# Foundation Layer
module "tags" {
  source = "../../modules/00-foundation/tags"

  env_name     = local.env_name
  project_name = "eyedar"
}

module "kms" {
  source = "../../modules/00-foundation/kms"

  env_name = local.env_name
  tags     = local.tags
}

module "secrets" {
  source = "../../modules/00-foundation/secrets"

  env_name    = local.env_name
  kms_key_arn = module.kms.kms_key_arn
  tags        = local.tags
}

# Network Layer
module "vpc" {
  source = "../../modules/10-network/vpc"

  env_name             = local.env_name
  vpc_cidr             = var.vpc_cidr
  availability_zones   = var.availability_zones
  public_subnet_cidrs  = var.public_subnet_cidrs
  private_subnet_cidrs = var.private_subnet_cidrs
  tags                 = local.tags
}

module "nat" {
  source = "../../modules/10-network/nat"

  env_name                = local.env_name
  vpc_id                  = module.vpc.vpc_id
  public_subnet_ids       = module.vpc.public_subnet_ids
  private_route_table_ids = module.vpc.private_route_table_ids
  nat_gateway_count       = var.nat_gateway_count
  tags                    = local.tags
}

module "endpoints" {
  source = "../../modules/10-network/endpoints"

  env_name                    = local.env_name
  vpc_id                      = module.vpc.vpc_id
  private_subnet_ids          = module.vpc.private_subnet_ids
  private_route_table_ids     = module.vpc.private_route_table_ids
  enable_s3_endpoint          = var.enable_s3_endpoint
  enable_interface_endpoints  = var.enable_interface_endpoints
  tags                        = local.tags
}

module "security_groups" {
  source = "../../modules/10-network/security_groups"

  env_name = local.env_name
  vpc_id   = module.vpc.vpc_id
  vpc_cidr = module.vpc.vpc_cidr
  tags     = local.tags
}

# Data Layer
module "s3_objects" {
  source = "../../modules/20-data/s3_objects"

  env_name    = local.env_name
  kms_key_arn = module.kms.kms_key_arn
  tags        = local.tags
}

module "rds" {
  source = "../../modules/20-data/rds_postgres"

  env_name             = local.env_name
  vpc_id               = module.vpc.vpc_id
  private_subnet_ids   = module.vpc.private_subnet_ids
  security_group_id    = module.security_groups.sg_rds_id
  kms_key_arn          = module.kms.kms_key_arn
  db_secret_arn        = module.secrets.db_secret_arn
  instance_class       = var.rds_instance_class
  allocated_storage    = var.rds_allocated_storage
  multi_az             = var.rds_multi_az
  backup_retention_days = 7  # 7-day retention for automated backups
  tags                 = local.tags
}

module "redis" {
  source = "../../modules/20-data/elasticache_redis"

  env_name           = local.env_name
  vpc_id             = module.vpc.vpc_id
  private_subnet_ids = module.vpc.private_subnet_ids
  security_group_id  = module.security_groups.sg_redis_id
  node_type          = var.redis_node_type
  num_cache_nodes    = var.redis_num_cache_nodes
  tags               = local.tags
}

module "sqs" {
  source = "../../modules/20-data/sqs_queues"

  env_name   = local.env_name
  kms_key_id = module.kms.kms_key_id
  tags       = local.tags
}

# Observability Layer
module "cloudwatch" {
  source = "../../modules/30-observability/cloudwatch"

  env_name          = local.env_name
  log_retention_days = var.log_retention_days
  kms_key_arn       = module.kms.kms_key_arn
  tags              = local.tags
}

module "budgets" {
  source = "../../modules/30-observability/budgets"

  env_name              = local.env_name
  monthly_budget_amount = var.monthly_budget_amount
  alert_emails          = var.budget_alert_emails
  tags                  = local.tags
}

# Compute Layer
module "ecr" {
  source = "../../modules/40-compute/ecr"

  env_name = local.env_name
  tags     = local.tags
}

module "ecs_cluster" {
  source = "../../modules/40-compute/ecs_cluster"

  env_name                   = local.env_name
  enable_container_insights  = var.enable_container_insights
  tags                       = local.tags
}

module "ecs_task_roles" {
  source = "../../modules/40-compute/ecs_task_roles"

  env_name      = local.env_name
  s3_bucket_arn = module.s3_objects.s3_bucket_arn
  sqs_queue_arns = {
    detection_created = module.sqs.queue_arn_detection_created
    verify_requested  = module.sqs.queue_arn_verify_requested
    verified_animals  = module.sqs.queue_arn_verified_animals
  }
  secret_arns = {
    db       = module.secrets.db_secret_arn
    firebase = module.secrets.firebase_secret_arn
    api_keys = module.secrets.api_keys_secret_arn
  }
  kms_key_arn = module.kms.kms_key_arn
  tags        = local.tags
}

module "ecs_services" {
  source = "../../modules/40-compute/ecs_services"

  env_name           = local.env_name
  ecs_cluster_arn    = module.ecs_cluster.ecs_cluster_arn
  private_subnet_ids = module.vpc.private_subnet_ids

  security_group_ids = {
    api       = module.security_groups.sg_ecs_api_id
    workers   = module.security_groups.sg_ecs_workers_id
    dashboard = module.security_groups.sg_ecs_dashboard_id
  }

  task_execution_role_arn = module.ecs_task_roles.task_execution_role_arn

  task_role_arns = {
    api            = module.ecs_task_roles.task_role_api_arn
    worker_ingest  = module.ecs_task_roles.task_role_worker_ingest_arn
    worker_verify  = module.ecs_task_roles.task_role_worker_verify_arn
    worker_notify  = module.ecs_task_roles.task_role_worker_notify_arn
    dashboard      = module.ecs_task_roles.task_role_dashboard_arn
  }

  log_group_names = {
    api            = module.cloudwatch.log_group_api_name
    worker_ingest  = module.cloudwatch.log_group_worker_ingest_name
    worker_verify  = module.cloudwatch.log_group_worker_verify_name
    worker_notify  = module.cloudwatch.log_group_worker_notify_name
    dashboard      = module.cloudwatch.log_group_dashboard_name
  }

  ecr_image_urls = {
    api            = module.ecr.ecr_repo_url_api
    worker_ingest  = module.ecr.ecr_repo_url_worker_ingest
    worker_verify  = module.ecr.ecr_repo_url_worker_verify
    worker_notify  = module.ecr.ecr_repo_url_worker_notify
    dashboard      = module.ecr.ecr_repo_url_dashboard
  }

  image_tags = {
    api            = var.image_tag_api
    worker_ingest  = var.image_tag_worker_ingest
    worker_verify  = var.image_tag_worker_verify
    worker_notify  = var.image_tag_worker_notify
    dashboard      = var.image_tag_dashboard
  }

  environment_vars = {
    rds_host                = module.rds.rds_address
    rds_port                = tostring(module.rds.rds_port)
    rds_db_name             = module.rds.rds_db_name
    redis_host              = module.redis.redis_endpoint
    redis_port              = tostring(module.redis.redis_port)
    s3_bucket_name          = module.s3_objects.s3_bucket_name
    sqs_queue_url_detection = module.sqs.queue_url_detection_created
    sqs_queue_url_verify    = module.sqs.queue_url_verify_requested
    sqs_queue_url_verified_animals = module.sqs.queue_url_verified_animals
  }

  secret_arns = {
    db       = module.secrets.db_secret_arn
    firebase = module.secrets.firebase_secret_arn
    api_keys = module.secrets.api_keys_secret_arn
  }

  api_desired_count           = var.api_desired_count
  worker_ingest_desired_count = var.worker_ingest_desired_count
  worker_verify_desired_count = var.worker_verify_desired_count
  worker_notify_desired_count = var.worker_notify_desired_count
  dashboard_desired_count     = var.dashboard_desired_count

  tags = local.tags
}

# Edge Layer
# ACM Certificate - ENABLED for HTTPS (required for production)
module "acm" {
  source = "../../modules/50-edge/acm"

  env_name       = local.env_name
  domain_name    = var.domain_name
  hosted_zone_id = var.hosted_zone_id
  tags           = local.tags
}

module "alb" {
  source = "../../modules/50-edge/alb"

  env_name            = local.env_name
  vpc_id              = module.vpc.vpc_id
  public_subnet_ids   = module.vpc.public_subnet_ids
  security_group_id   = module.security_groups.sg_alb_public_id
  #acm_certificate_arn = module.acm.certificate_arn  # HTTPS enabled

  api_target_config = {
    port = module.ecs_services.api_port
  }

  dashboard_target_config = {
    port = module.ecs_services.dashboard_port
  }

  tags = local.tags
}

# Note: ECS services with ALB are managed by load_balancer configuration in the service definition
# Manual target group attachments are not required for Fargate services

module "waf" {
  count  = var.enable_waf ? 1 : 0
  source = "../../modules/50-edge/waf"

  env_name           = local.env_name
  alb_arn            = module.alb.alb_arn
  enable_rate_limiting = true
  rate_limit         = var.waf_rate_limit
  tags               = local.tags
}

# CI/CD Layer
module "github_oidc" {
  source = "../../modules/60-cicd/github_oidc"

  env_name   = local.env_name
  github_org = var.github_org
  github_repo = var.github_repo
  github_branches = var.github_branches

  ecr_repository_arns = [
    module.ecr.ecr_repo_arn_api,
    module.ecr.ecr_repo_arn_worker_ingest,
    module.ecr.ecr_repo_arn_worker_verify,
    module.ecr.ecr_repo_arn_dashboard
  ]

  ecs_cluster_arn = module.ecs_cluster.ecs_cluster_arn

  ecs_service_names = [
    module.ecs_services.api_service_name,
    module.ecs_services.worker_ingest_service_name
  ]

  tags = local.tags
}
