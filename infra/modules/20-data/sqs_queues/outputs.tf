output "queue_url_detection_created" {
  description = "URL of the detection_created queue"
  value       = aws_sqs_queue.detection_created.url
}

output "queue_arn_detection_created" {
  description = "ARN of the detection_created queue"
  value       = aws_sqs_queue.detection_created.arn
}

output "dlq_url_detection_created" {
  description = "URL of the detection_created DLQ"
  value       = aws_sqs_queue.detection_created_dlq.url
}

output "dlq_arn_detection_created" {
  description = "ARN of the detection_created DLQ"
  value       = aws_sqs_queue.detection_created_dlq.arn
}

output "queue_url_verify_requested" {
  description = "URL of the verify_requested queue"
  value       = aws_sqs_queue.verify_requested.url
}

output "queue_arn_verify_requested" {
  description = "ARN of the verify_requested queue"
  value       = aws_sqs_queue.verify_requested.arn
}

output "dlq_url_verify_requested" {
  description = "URL of the verify_requested DLQ"
  value       = aws_sqs_queue.verify_requested_dlq.url
}

output "dlq_arn_verify_requested" {
  description = "ARN of the verify_requested DLQ"
  value       = aws_sqs_queue.verify_requested_dlq.arn
}

output "queue_url_verified_animals" {
  description = "URL of the verified_animals queue"
  value       = aws_sqs_queue.verified_animals.url
}

output "queue_arn_verified_animals" {
  description = "ARN of the verified_animals queue"
  value       = aws_sqs_queue.verified_animals.arn
}

output "dlq_url_verified_animals" {
  description = "URL of the verified_animals DLQ"
  value       = aws_sqs_queue.verified_animals_dlq.url
}

output "dlq_arn_verified_animals" {
  description = "ARN of the verified_animals DLQ"
  value       = aws_sqs_queue.verified_animals_dlq.arn
}
