# Log Groups for ECS Services
resource "aws_cloudwatch_log_group" "api" {
  name              = "/ecs/eyedar-${var.env_name}-api"
  retention_in_days = var.log_retention_days
  kms_key_id        = var.kms_key_arn

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-api-logs"
    Service = "api"
  })
}

resource "aws_cloudwatch_log_group" "worker_ingest" {
  name              = "/ecs/eyedar-${var.env_name}-worker-ingest"
  retention_in_days = var.log_retention_days
  kms_key_id        = var.kms_key_arn

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-ingest-logs"
    Service = "worker-ingest"
  })
}

resource "aws_cloudwatch_log_group" "worker_verify" {
  name              = "/ecs/eyedar-${var.env_name}-worker-verify"
  retention_in_days = var.log_retention_days
  kms_key_id        = var.kms_key_arn

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-verify-logs"
    Service = "worker-verify"
  })
}

resource "aws_cloudwatch_log_group" "dashboard" {
  name              = "/ecs/eyedar-${var.env_name}-dashboard"
  retention_in_days = var.log_retention_days
  kms_key_id        = var.kms_key_arn

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-dashboard-logs"
    Service = "dashboard"
  })
}

resource "aws_cloudwatch_log_group" "worker_notify" {
  name              = "/ecs/eyedar-${var.env_name}-worker-notify"
  retention_in_days = var.log_retention_days
  kms_key_id        = var.kms_key_arn

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-notify-logs"
    Service = "worker-notify"
  })
}

resource "aws_cloudwatch_log_group" "rust_api" {
  name              = "/ecs/eyedar-${var.env_name}-rust-api"
  retention_in_days = var.log_retention_days
  kms_key_id        = var.kms_key_arn

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-rust-api-logs"
    Service = "rust-api"
  })
}

resource "aws_cloudwatch_log_group" "scheduler" {
  name              = "/ecs/eyedar-${var.env_name}-scheduler"
  retention_in_days = var.log_retention_days
  kms_key_id        = var.kms_key_arn

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-scheduler-logs"
    Service = "scheduler"
  })
}

# Log Group for ALB
resource "aws_cloudwatch_log_group" "alb" {
  name              = "/aws/alb/eyedar-${var.env_name}"
  retention_in_days = var.log_retention_days
  kms_key_id        = var.kms_key_arn

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-alb-logs"
    Service = "alb"
  })
}

# SNS Topic for Alarms (if not provided)
resource "aws_sns_topic" "alarms" {
  count = var.alarm_sns_topic_arn == null ? 1 : 0
  name  = "eyedar-${var.env_name}-alarms"

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-alarms"
  })
}

# Example CloudWatch Dashboard
resource "aws_cloudwatch_dashboard" "main" {
  dashboard_name = "eyedar-${var.env_name}-overview"

  dashboard_body = jsonencode({
    widgets = [
      {
        type = "metric"
        properties = {
          metrics = [
            ["AWS/ECS", "CPUUtilization", { stat = "Average" }],
            [".", "MemoryUtilization", { stat = "Average" }]
          ]
          period = 300
          stat   = "Average"
          region = data.aws_region.current.name
          title  = "ECS Resource Utilization"
        }
      },
      {
        type = "metric"
        properties = {
          metrics = [
            ["AWS/SQS", "ApproximateNumberOfMessagesVisible", { stat = "Sum" }],
            [".", "NumberOfMessagesSent", { stat = "Sum" }],
            [".", "NumberOfMessagesReceived", { stat = "Sum" }]
          ]
          period = 300
          stat   = "Sum"
          region = data.aws_region.current.name
          title  = "SQS Queue Metrics"
        }
      }
    ]
  })
}

data "aws_region" "current" {}
