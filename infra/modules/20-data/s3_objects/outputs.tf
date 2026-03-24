output "s3_bucket_name" {
  description = "Name of the S3 bucket"
  value       = aws_s3_bucket.objects.id
}

output "s3_bucket_arn" {
  description = "ARN of the S3 bucket"
  value       = aws_s3_bucket.objects.arn
}

output "s3_bucket_domain_name" {
  description = "Domain name of the S3 bucket"
  value       = aws_s3_bucket.objects.bucket_domain_name
}
