output "alb_id" {
  description = "ID of the load balancer"
  value       = aws_lb.main.id
}

output "alb_arn" {
  description = "ARN of the load balancer"
  value       = aws_lb.main.arn
}

output "alb_dns_name" {
  description = "DNS name of the load balancer"
  value       = aws_lb.main.dns_name
}

output "alb_zone_id" {
  description = "Zone ID of the load balancer"
  value       = aws_lb.main.zone_id
}

output "target_group_api_arn" {
  description = "ARN of the API target group"
  value       = aws_lb_target_group.api.arn
}

output "http_listener_arn" {
  description = "ARN of the HTTP listener"
  value       = aws_lb_listener.http.arn
}

# HTTPS listener output - disabled for HTTP-only mode
# output "https_listener_arn" {
#   description = "ARN of the HTTPS listener"
#   value       = aws_lb_listener.https.arn
# }
