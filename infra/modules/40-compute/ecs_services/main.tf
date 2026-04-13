data "aws_region" "current" {}

# API Service
resource "aws_ecs_task_definition" "api" {
  family                   = "eyedar-${var.env_name}-api"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = var.task_execution_role_arn
  task_role_arn            = var.task_role_arns.api

  container_definitions = jsonencode([{
    name  = "api"
    image = "${var.ecr_image_urls.api}:${var.image_tags.api}"

    portMappings = [{
      containerPort = 3000
      protocol      = "tcp"
    }]

    environment = [
      { name = "NODE_ENV", value = var.env_name },
      { name = "PORT", value = "3000" },
      { name = "DB_HOST", value = var.environment_vars.rds_host },
      { name = "DB_PORT", value = var.environment_vars.rds_port },
      { name = "DB_NAME", value = var.environment_vars.rds_db_name },
      { name = "REDIS_HOST", value = var.environment_vars.redis_host },
      { name = "REDIS_PORT", value = var.environment_vars.redis_port },
      { name = "S3_BUCKET", value = var.environment_vars.s3_bucket_name },
      { name = "SQS_QUEUE_URL_DETECTION", value = var.environment_vars.sqs_queue_url_detection },
      { name = "AWS_REGION", value = data.aws_region.current.name }
    ]

    secrets = [
      { name = "DB_PASSWORD", valueFrom = "${var.secret_arns.db}:password::" },
      { name = "DB_USERNAME", valueFrom = "${var.secret_arns.db}:username::" },
      { name = "FIREBASE_CONFIG", valueFrom = var.secret_arns.firebase }
    ]

    logConfiguration = {
      logDriver = "awslogs"
      options = {
        "awslogs-group"         = var.log_group_names.api
        "awslogs-region"        = data.aws_region.current.name
        "awslogs-stream-prefix" = "api"
      }
    }

    healthCheck = {
      command     = ["CMD-SHELL", "curl -f http://localhost:3000/health || exit 1"]
      interval    = 30
      timeout     = 5
      retries     = 3
      startPeriod = 60
    }
  }])

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-api"
    Service = "api"
  })
}

resource "aws_ecs_service" "api" {
  name            = "eyedar-${var.env_name}-api"
  cluster         = var.ecs_cluster_arn
  task_definition = aws_ecs_task_definition.api.arn
  desired_count   = var.api_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [var.security_group_ids.api]
    assign_public_ip = false
  }

  deployment_maximum_percent         = 200
  deployment_minimum_healthy_percent = 100

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-api"
    Service = "api"
  })

  lifecycle {
    ignore_changes = [desired_count]
  }
}

# Rust API Service (Primary entry point for ESP devices)
resource "aws_ecs_task_definition" "rust_api" {
  family                   = "eyedar-${var.env_name}-rust-api"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = var.task_execution_role_arn
  task_role_arn            = var.task_role_arns.rust_api

  container_definitions = jsonencode([{
    name  = "rust-api"
    image = "${var.ecr_image_urls.rust_api}:${var.image_tags.rust_api}"

    portMappings = [{
      containerPort = 8080
      protocol      = "tcp"
    }]

    environment = [
      { name = "RUST_LOG", value = "info" },
      { name = "PORT", value = "8080" },
      { name = "DB_HOST", value = var.environment_vars.rds_host },
      { name = "DB_PORT", value = var.environment_vars.rds_port },
      { name = "DB_NAME", value = var.environment_vars.rds_db_name },
      { name = "S3_BUCKET", value = var.environment_vars.s3_bucket_name },
      { name = "S3_ENDPOINT", value = "https://s3.${data.aws_region.current.name}.amazonaws.com" },
      { name = "REDIS_HOST", value = var.environment_vars.redis_host },
      { name = "REDIS_PORT", value = var.environment_vars.redis_port },
      { name = "REDIS_ENDPOINT", value = "redis://${var.environment_vars.redis_host}:${var.environment_vars.redis_port}" },
      { name = "SQS_QUEUE_URL", value = var.environment_vars.sqs_queue_url_detection },
      { name = "AWS_REGION", value = data.aws_region.current.name }
    ]

    secrets = [
      { name = "API_KEY", valueFrom = var.secret_arns.api_keys },
      { name = "DB_USERNAME", valueFrom = "${var.secret_arns.db}:username::" },
      { name = "DB_PASSWORD", valueFrom = "${var.secret_arns.db}:password::" }
    ]

    logConfiguration = {
      logDriver = "awslogs"
      options = {
        "awslogs-group"         = var.log_group_names.rust_api
        "awslogs-region"        = data.aws_region.current.name
        "awslogs-stream-prefix" = "rust-api"
      }
    }

    healthCheck = {
      command     = ["CMD-SHELL", "curl -f http://localhost:8080/health || exit 1"]
      interval    = 30
      timeout     = 5
      retries     = 3
      startPeriod = 60
    }
  }])

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-rust-api"
    Service = "rust-api"
  })
}

resource "aws_ecs_service" "rust_api" {
  count           = var.rust_api_desired_count > 0 ? 1 : 0
  name            = "eyedar-${var.env_name}-rust-api"
  cluster         = var.ecs_cluster_arn
  task_definition = aws_ecs_task_definition.rust_api.arn
  desired_count   = var.rust_api_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [var.security_group_ids.api]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = var.alb_target_group_arn_api
    container_name   = "rust-api"
    container_port   = 8080
  }

  deployment_maximum_percent         = 200
  deployment_minimum_healthy_percent = 100

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-rust-api"
    Service = "rust-api"
  })

  lifecycle {
    ignore_changes = [desired_count]
  }
}


resource "aws_ecs_task_definition" "worker_ingest" {
  family                   = "eyedar-${var.env_name}-worker-ingest"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = var.task_execution_role_arn
  task_role_arn            = var.task_role_arns.worker_ingest

  container_definitions = jsonencode([{
    name  = "worker-ingest"
    image = "${var.ecr_image_urls.worker_ingest}:${var.image_tags.worker_ingest}"

    environment = [
      { name = "NODE_ENV", value = var.env_name },
      { name = "DB_HOST", value = var.environment_vars.rds_host },
      { name = "DB_PORT", value = var.environment_vars.rds_port },
      { name = "DB_NAME", value = var.environment_vars.rds_db_name },
      { name = "REDIS_HOST", value = var.environment_vars.redis_host },
      { name = "REDIS_PORT", value = var.environment_vars.redis_port },
      { name = "SQS_QUEUE_URL_DETECTION", value = var.environment_vars.sqs_queue_url_detection },
      { name = "AWS_REGION", value = data.aws_region.current.name }
    ]

    secrets = [
      { name = "DB_PASSWORD", valueFrom = "${var.secret_arns.db}:password::" },
      { name = "DB_USERNAME", valueFrom = "${var.secret_arns.db}:username::" }
    ]

    logConfiguration = {
      logDriver = "awslogs"
      options = {
        "awslogs-group"         = var.log_group_names.worker_ingest
        "awslogs-region"        = data.aws_region.current.name
        "awslogs-stream-prefix" = "worker-ingest"
      }
    }
  }])

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-ingest"
    Service = "worker-ingest"
  })
}

resource "aws_ecs_service" "worker_ingest" {
  name            = "eyedar-${var.env_name}-worker-ingest"
  cluster         = var.ecs_cluster_arn
  task_definition = aws_ecs_task_definition.worker_ingest.arn
  desired_count   = var.worker_ingest_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [var.security_group_ids.workers]
    assign_public_ip = false
  }

  deployment_maximum_percent         = 200
  deployment_minimum_healthy_percent = 100

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-ingest"
    Service = "worker-ingest"
  })

  lifecycle {
    ignore_changes = [desired_count]
  }
}

# Worker Verify Service
resource "aws_ecs_task_definition" "worker_verify" {
  family                   = "eyedar-${var.env_name}-worker-verify"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "512"  # More CPU for ML
  memory                   = "1024" # More memory for ML
  execution_role_arn       = var.task_execution_role_arn
  task_role_arn            = var.task_role_arns.worker_verify

  container_definitions = jsonencode([{
    name  = "worker-verify"
    image = "${var.ecr_image_urls.worker_verify}:${var.image_tags.worker_verify}"

    environment = [
      { name = "NODE_ENV", value = var.env_name },
      { name = "DB_HOST", value = var.environment_vars.rds_host },
      { name = "DB_PORT", value = var.environment_vars.rds_port },
      { name = "DB_NAME", value = var.environment_vars.rds_db_name },
      { name = "S3_BUCKET", value = var.environment_vars.s3_bucket_name },
      { name = "SQS_QUEUE_URL_VERIFY", value = var.environment_vars.sqs_queue_url_verify },
      { name = "AWS_REGION", value = data.aws_region.current.name }
    ]

    secrets = [
      { name = "DB_PASSWORD", valueFrom = "${var.secret_arns.db}:password::" },
      { name = "DB_USERNAME", valueFrom = "${var.secret_arns.db}:username::" }
    ]

    logConfiguration = {
      logDriver = "awslogs"
      options = {
        "awslogs-group"         = var.log_group_names.worker_verify
        "awslogs-region"        = data.aws_region.current.name
        "awslogs-stream-prefix" = "worker-verify"
      }
    }
  }])

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-verify"
    Service = "worker-verify"
  })
}

resource "aws_ecs_service" "worker_verify" {
  count           = var.worker_verify_desired_count > 0 ? 1 : 0
  name            = "eyedar-${var.env_name}-worker-verify"
  cluster         = var.ecs_cluster_arn
  task_definition = aws_ecs_task_definition.worker_verify.arn
  desired_count   = var.worker_verify_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [var.security_group_ids.workers]
    assign_public_ip = false
  }

  deployment_maximum_percent         = 200
  deployment_minimum_healthy_percent = 100

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-verify"
    Service = "worker-verify"
  })

  lifecycle {
    ignore_changes = [desired_count]
  }
}

# Dashboard Service
resource "aws_ecs_task_definition" "dashboard" {
  family                   = "eyedar-${var.env_name}-dashboard"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = var.task_execution_role_arn
  task_role_arn            = var.task_role_arns.dashboard

  container_definitions = jsonencode([{
    name  = "dashboard"
    image = "${var.ecr_image_urls.dashboard}:${var.image_tags.dashboard}"

    portMappings = [{
      containerPort = 3000
      protocol      = "tcp"
    }]

    environment = [
      { name = "NODE_ENV", value = var.env_name },
      { name = "PORT", value = "3000" },
      { name = "DB_HOST", value = var.environment_vars.rds_host },
      { name = "DB_PORT", value = var.environment_vars.rds_port },
      { name = "DB_NAME", value = var.environment_vars.rds_db_name },
      { name = "AWS_REGION", value = data.aws_region.current.name }
    ]

    secrets = [
      { name = "DB_PASSWORD", valueFrom = "${var.secret_arns.db}:password::" },
      { name = "DB_USERNAME", valueFrom = "${var.secret_arns.db}:username::" }
    ]

    logConfiguration = {
      logDriver = "awslogs"
      options = {
        "awslogs-group"         = var.log_group_names.dashboard
        "awslogs-region"        = data.aws_region.current.name
        "awslogs-stream-prefix" = "dashboard"
      }
    }

    healthCheck = {
      command     = ["CMD-SHELL", "curl -f http://localhost:3000/health || exit 1"]
      interval    = 30
      timeout     = 5
      retries     = 3
      startPeriod = 60
    }
  }])

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-dashboard"
    Service = "dashboard"
  })
}

resource "aws_ecs_service" "dashboard" {
  count           = var.dashboard_desired_count > 0 ? 1 : 0
  name            = "eyedar-${var.env_name}-dashboard"
  cluster         = var.ecs_cluster_arn
  task_definition = aws_ecs_task_definition.dashboard.arn
  desired_count   = var.dashboard_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [var.security_group_ids.dashboard]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = var.alb_target_group_arn_dashboard
    container_name   = "dashboard"
    container_port   = 3000
  }

  deployment_maximum_percent         = 200
  deployment_minimum_healthy_percent = 100

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-dashboard"
    Service = "dashboard"
  })

  lifecycle {
    ignore_changes = [desired_count]
  }
}

# Worker-Notify Service
resource "aws_ecs_task_definition" "worker_notify" {
  family                   = "eyedar-${var.env_name}-worker-notify"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = var.task_execution_role_arn
  task_role_arn            = var.task_role_arns.worker_notify

  container_definitions = jsonencode([{
    name  = "worker-notify"
    image = "${var.ecr_image_urls.worker_notify}:${var.image_tags.worker_notify}"

    environment = [
      { name = "NODE_ENV", value = var.env_name },
      { name = "DB_HOST", value = var.environment_vars.rds_host },
      { name = "DB_PORT", value = var.environment_vars.rds_port },
      { name = "DB_NAME", value = var.environment_vars.rds_db_name },
      { name = "SQS_QUEUE_URL_VERIFIED_ANIMALS", value = var.environment_vars.sqs_queue_url_verified_animals },
      { name = "AWS_REGION", value = data.aws_region.current.name }
    ]

    secrets = [
      { name = "DB_PASSWORD", valueFrom = "${var.secret_arns.db}:password::" },
      { name = "DB_USERNAME", valueFrom = "${var.secret_arns.db}:username::" },
      { name = "FIREBASE_SERVICE_ACCOUNT", valueFrom = var.secret_arns.firebase }
    ]

    logConfiguration = {
      logDriver = "awslogs"
      options = {
        "awslogs-group"         = var.log_group_names.worker_notify
        "awslogs-region"        = data.aws_region.current.name
        "awslogs-stream-prefix" = "ecs"
      }
    }
  }])

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-notify"
    Service = "worker-notify"
  })
}

resource "aws_ecs_service" "worker_notify" {
  count           = var.worker_notify_desired_count > 0 ? 1 : 0
  name            = "eyedar-${var.env_name}-worker-notify"
  cluster         = var.ecs_cluster_arn
  task_definition = aws_ecs_task_definition.worker_notify.arn
  desired_count   = var.worker_notify_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [var.security_group_ids.workers]
    assign_public_ip = false
  }

  deployment_maximum_percent         = 200
  deployment_minimum_healthy_percent = 100

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-notify"
    Service = "worker-notify"
  })

  lifecycle {
    ignore_changes = [desired_count]
  }
}
