# S3 Bucket for detection images (private)
resource "aws_s3_bucket" "objects" {
  bucket        = "eyedar-${var.env_name}-objects"
  force_destroy = var.force_destroy

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-objects"
  })
}

# Block all public access
resource "aws_s3_bucket_public_access_block" "objects" {
  bucket = aws_s3_bucket.objects.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Enable versioning
resource "aws_s3_bucket_versioning" "objects" {
  bucket = aws_s3_bucket.objects.id

  versioning_configuration {
    status = "Enabled"
  }
}

# Encryption configuration
resource "aws_s3_bucket_server_side_encryption_configuration" "objects" {
  bucket = aws_s3_bucket.objects.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = "aws:kms"
      kms_master_key_id = var.kms_key_arn
    }
    bucket_key_enabled = true
  }
}

# Lifecycle policy for cost optimization
resource "aws_s3_bucket_lifecycle_configuration" "objects" {
  count  = var.lifecycle_enabled ? 1 : 0
  bucket = aws_s3_bucket.objects.id

  rule {
    id     = "transition-to-ia"
    status = "Enabled"

    filter {}

    transition {
      days          = 30
      storage_class = "STANDARD_IA"
    }

    transition {
      days          = 90
      storage_class = "GLACIER_IR"
    }

    # Optional: expire old objects after 365 days
    expiration {
      days = 365
    }
  }

  rule {
    id     = "delete-incomplete-uploads"
    status = "Enabled"

    filter {}

    abort_incomplete_multipart_upload {
      days_after_initiation = 7
    }
  }
}

# Bucket policy - restrict access to specific IAM roles only (will be added via task roles)
resource "aws_s3_bucket_policy" "objects" {
  bucket = aws_s3_bucket.objects.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "DenyInsecureTransport"
        Effect = "Deny"
        Principal = "*"
        Action = "s3:*"
        Resource = [
          aws_s3_bucket.objects.arn,
          "${aws_s3_bucket.objects.arn}/*"
        ]
        Condition = {
          Bool = {
            "aws:SecureTransport" = "false"
          }
        }
      }
    ]
  })
}
