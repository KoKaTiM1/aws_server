# Dead Letter Queue for detection_created
resource "aws_sqs_queue" "detection_created_dlq" {
  name = "eyedar-${var.env_name}-detection-created-dlq"

  message_retention_seconds = 1209600 # 14 days
  kms_master_key_id         = var.kms_key_id
  kms_data_key_reuse_period_seconds = 300

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-detection-created-dlq"
  })
}

# Main Queue for detection_created
resource "aws_sqs_queue" "detection_created" {
  name = "eyedar-${var.env_name}-detection-created"

  message_retention_seconds = var.message_retention_seconds
  visibility_timeout_seconds = var.visibility_timeout_seconds
  kms_master_key_id         = var.kms_key_id
  kms_data_key_reuse_period_seconds = 300

  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.detection_created_dlq.arn
    maxReceiveCount     = var.max_receive_count
  })

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-detection-created"
  })
}

# Dead Letter Queue for verify_requested
resource "aws_sqs_queue" "verify_requested_dlq" {
  name = "eyedar-${var.env_name}-verify-requested-dlq"

  message_retention_seconds = 1209600 # 14 days
  kms_master_key_id         = var.kms_key_id
  kms_data_key_reuse_period_seconds = 300

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-verify-requested-dlq"
  })
}

# Main Queue for verify_requested (optional ML verification worker)
resource "aws_sqs_queue" "verify_requested" {
  name = "eyedar-${var.env_name}-verify-requested"

  message_retention_seconds = var.message_retention_seconds
  visibility_timeout_seconds = 300 # Longer for ML processing
  kms_master_key_id         = var.kms_key_id
  kms_data_key_reuse_period_seconds = 300

  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.verify_requested_dlq.arn
    maxReceiveCount     = var.max_receive_count
  })

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-verify-requested"
  })
}

# CloudWatch Alarms for DLQ (alerts on messages in dead letter queues)
resource "aws_cloudwatch_metric_alarm" "detection_dlq_alarm" {
  alarm_name          = "eyedar-${var.env_name}-detection-dlq-messages"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "ApproximateNumberOfMessagesVisible"
  namespace           = "AWS/SQS"
  period              = 300
  statistic           = "Average"
  threshold           = 0
  alarm_description   = "Alert when messages appear in detection DLQ"
  treat_missing_data  = "notBreaching"

  dimensions = {
    QueueName = aws_sqs_queue.detection_created_dlq.name
  }

  tags = var.tags
}

resource "aws_cloudwatch_metric_alarm" "verify_dlq_alarm" {
  alarm_name          = "eyedar-${var.env_name}-verify-dlq-messages"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "ApproximateNumberOfMessagesVisible"
  namespace           = "AWS/SQS"
  period              = 300
  statistic           = "Average"
  threshold           = 0
  alarm_description   = "Alert when messages appear in verify DLQ"
  treat_missing_data  = "notBreaching"

  dimensions = {
    QueueName = aws_sqs_queue.verify_requested_dlq.name
  }

  tags = var.tags
}

# Dead Letter Queue for verified_animals
resource "aws_sqs_queue" "verified_animals_dlq" {
  name = "eyedar-${var.env_name}-verified-animals-dlq"

  message_retention_seconds = 1209600 # 14 days
  kms_master_key_id         = var.kms_key_id
  kms_data_key_reuse_period_seconds = 300

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-verified-animals-dlq"
  })
}

# Main Queue for verified_animals (for Worker-Notify to send FCM notifications)
resource "aws_sqs_queue" "verified_animals" {
  name = "eyedar-${var.env_name}-verified-animals"

  message_retention_seconds = var.message_retention_seconds
  visibility_timeout_seconds = var.visibility_timeout_seconds
  kms_master_key_id         = var.kms_key_id
  kms_data_key_reuse_period_seconds = 300

  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.verified_animals_dlq.arn
    maxReceiveCount     = var.max_receive_count
  })

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-verified-animals"
  })
}

# CloudWatch Alarm for verified_animals DLQ
resource "aws_cloudwatch_metric_alarm" "verified_animals_dlq_alarm" {
  alarm_name          = "eyedar-${var.env_name}-verified-animals-dlq-messages"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "ApproximateNumberOfMessagesVisible"
  namespace           = "AWS/SQS"
  period              = 300
  statistic           = "Average"
  threshold           = 0
  alarm_description   = "Alert when messages appear in verified animals DLQ"
  treat_missing_data  = "notBreaching"

  dimensions = {
    QueueName = aws_sqs_queue.verified_animals_dlq.name
  }

  tags = var.tags
}
