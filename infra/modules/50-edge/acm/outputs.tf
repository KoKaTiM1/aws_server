output "acm_certificate_arn" {
  description = "ARN of the ACM certificate"
  value       = aws_acm_certificate.main.arn
}

output "acm_certificate_domain" {
  description = "Domain name of the certificate"
  value       = aws_acm_certificate.main.domain_name
}

output "acm_certificate_status" {
  description = "Status of the certificate"
  value       = aws_acm_certificate.main.status
}
