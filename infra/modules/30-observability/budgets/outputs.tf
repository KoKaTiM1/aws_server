output "budget_name" {
  description = "Name of the budget"
  value       = aws_budgets_budget.monthly.name
}

output "budget_sns_topic_arn" {
  description = "ARN of the SNS topic for budget alerts"
  value       = aws_sns_topic.budget_alerts.arn
}

output "budget_amount" {
  description = "Monthly budget amount"
  value       = var.monthly_budget_amount
}
