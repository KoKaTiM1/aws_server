# SNS Topic for Budget Alerts
resource "aws_sns_topic" "budget_alerts" {
  name = "eyedar-${var.env_name}-budget-alerts"

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-budget-alerts"
  })
}

# SNS Topic Subscriptions
resource "aws_sns_topic_subscription" "budget_email" {
  count     = length(var.alert_emails)
  topic_arn = aws_sns_topic.budget_alerts.arn
  protocol  = "email"
  endpoint  = var.alert_emails[count.index]
}

# Monthly Budget
resource "aws_budgets_budget" "monthly" {
  name              = "eyedar-${var.env_name}-monthly-budget"
  budget_type       = "COST"
  limit_amount      = tostring(var.monthly_budget_amount)
  limit_unit        = "USD"
  time_unit         = "MONTHLY"
  time_period_start = "2024-01-01_00:00"

  cost_filter {
    name = "TagKeyValue"
    values = [
      "user:Environment$${var.env_name}"
    ]
  }

  # Create notifications for each threshold
  dynamic "notification" {
    for_each = var.alert_threshold_percentages
    content {
      comparison_operator        = "GREATER_THAN"
      threshold                  = notification.value
      threshold_type             = "PERCENTAGE"
      notification_type          = "ACTUAL"
      subscriber_sns_topic_arns  = [aws_sns_topic.budget_alerts.arn]
    }
  }

  # Forecasted cost alerts
  notification {
    comparison_operator        = "GREATER_THAN"
    threshold                  = 100
    threshold_type             = "PERCENTAGE"
    notification_type          = "FORECASTED"
    subscriber_sns_topic_arns  = [aws_sns_topic.budget_alerts.arn]
  }
}
