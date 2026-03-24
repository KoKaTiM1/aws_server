# ECS Task Execution Role (for all services)
# Allows ECS agent to pull images, write logs, fetch secrets
resource "aws_iam_role" "ecs_task_execution" {
  name = "eyedar-${var.env_name}-ecs-task-execution"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
    }]
  })

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-ecs-task-execution"
  })
}

resource "aws_iam_role_policy_attachment" "ecs_task_execution" {
  role       = aws_iam_role.ecs_task_execution.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

# Allow execution role to read secrets and decrypt with KMS
resource "aws_iam_role_policy" "ecs_task_execution_secrets" {
  name = "secrets-access"
  role = aws_iam_role.ecs_task_execution.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          var.secret_arns.db,
          var.secret_arns.firebase,
          var.secret_arns.api_keys
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt",
          "kms:DescribeKey"
        ]
        Resource = var.kms_key_arn
      }
    ]
  })
}

# API Service Task Role
resource "aws_iam_role" "api" {
  name = "eyedar-${var.env_name}-api-task"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
    }]
  })

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-api-task"
    Service = "api"
  })
}

resource "aws_iam_role_policy" "api" {
  name = "api-permissions"
  role = aws_iam_role.api.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:PutObject",
          "s3:GetObject",
          "s3:DeleteObject"
        ]
        Resource = "${var.s3_bucket_arn}/*"
      },
      {
        Effect = "Allow"
        Action = [
          "sqs:SendMessage",
          "sqs:GetQueueAttributes"
        ]
        Resource = [
          var.sqs_queue_arns.detection_created,
          var.sqs_queue_arns.verify_requested
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          var.secret_arns.db,
          var.secret_arns.firebase,
          var.secret_arns.api_keys
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt",
          "kms:GenerateDataKey"
        ]
        Resource = var.kms_key_arn
      }
    ]
  })
}

# Worker Ingest Task Role
resource "aws_iam_role" "worker_ingest" {
  name = "eyedar-${var.env_name}-worker-ingest-task"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
    }]
  })

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-ingest-task"
    Service = "worker-ingest"
  })
}

resource "aws_iam_role_policy" "worker_ingest" {
  name = "worker-ingest-permissions"
  role = aws_iam_role.worker_ingest.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "sqs:ReceiveMessage",
          "sqs:DeleteMessage",
          "sqs:GetQueueAttributes",
          "sqs:ChangeMessageVisibility"
        ]
        Resource = var.sqs_queue_arns.detection_created
      },
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          var.secret_arns.db
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt"
        ]
        Resource = var.kms_key_arn
      }
    ]
  })
}

# Worker Verify Task Role
resource "aws_iam_role" "worker_verify" {
  name = "eyedar-${var.env_name}-worker-verify-task"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
    }]
  })

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-verify-task"
    Service = "worker-verify"
  })
}

resource "aws_iam_role_policy" "worker_verify" {
  name = "worker-verify-permissions"
  role = aws_iam_role.worker_verify.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "sqs:ReceiveMessage",
          "sqs:DeleteMessage",
          "sqs:GetQueueAttributes",
          "sqs:ChangeMessageVisibility"
        ]
        Resource = var.sqs_queue_arns.verify_requested
      },
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject"
        ]
        Resource = "${var.s3_bucket_arn}/*"
      },
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          var.secret_arns.db
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt"
        ]
        Resource = var.kms_key_arn
      }
    ]
  })
}

# Dashboard Task Role
resource "aws_iam_role" "dashboard" {
  name = "eyedar-${var.env_name}-dashboard-task"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
    }]
  })

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-dashboard-task"
    Service = "dashboard"
  })
}

resource "aws_iam_role_policy" "dashboard" {
  name = "dashboard-permissions"
  role = aws_iam_role.dashboard.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          var.secret_arns.db,
          var.secret_arns.api_keys
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt"
        ]
        Resource = var.kms_key_arn
      }
    ]
  })
}

# Worker-Notify Task Role (sends FCM notifications)
resource "aws_iam_role" "worker_notify" {
  name = "eyedar-${var.env_name}-worker-notify-task"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
    }]
  })

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-notify-task"
    Service = "worker-notify"
  })
}

resource "aws_iam_role_policy" "worker_notify" {
  name = "worker-notify-permissions"
  role = aws_iam_role.worker_notify.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "sqs:ReceiveMessage",
          "sqs:DeleteMessage",
          "sqs:GetQueueAttributes",
          "sqs:ChangeMessageVisibility"
        ]
        Resource = var.sqs_queue_arns.verified_animals
      },
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          var.secret_arns.db,
          var.secret_arns.firebase
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt"
        ]
        Resource = var.kms_key_arn
      }
    ]
  })
}
