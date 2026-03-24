# ECR Repository for API
resource "aws_ecr_repository" "api" {
  name                 = "eyedar-${var.env_name}-api"
  image_tag_mutability = var.image_tag_mutability

  image_scanning_configuration {
    scan_on_push = var.scan_on_push
  }

  encryption_configuration {
    encryption_type = "AES256"
  }

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-api"
    Service = "api"
  })
}

# Lifecycle policy for API
resource "aws_ecr_lifecycle_policy" "api" {
  repository = aws_ecr_repository.api.name

  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep last ${var.retention_count} images"
      selection = {
        tagStatus   = "any"
        countType   = "imageCountMoreThan"
        countNumber = var.retention_count
      }
      action = {
        type = "expire"
      }
    }]
  })
}

# ECR Repository for Worker Ingest
resource "aws_ecr_repository" "worker_ingest" {
  name                 = "eyedar-${var.env_name}-worker-ingest"
  image_tag_mutability = var.image_tag_mutability

  image_scanning_configuration {
    scan_on_push = var.scan_on_push
  }

  encryption_configuration {
    encryption_type = "AES256"
  }

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-ingest"
    Service = "worker-ingest"
  })
}

resource "aws_ecr_lifecycle_policy" "worker_ingest" {
  repository = aws_ecr_repository.worker_ingest.name

  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep last ${var.retention_count} images"
      selection = {
        tagStatus   = "any"
        countType   = "imageCountMoreThan"
        countNumber = var.retention_count
      }
      action = {
        type = "expire"
      }
    }]
  })
}

# ECR Repository for Worker Verify
resource "aws_ecr_repository" "worker_verify" {
  name                 = "eyedar-${var.env_name}-worker-verify"
  image_tag_mutability = var.image_tag_mutability

  image_scanning_configuration {
    scan_on_push = var.scan_on_push
  }

  encryption_configuration {
    encryption_type = "AES256"
  }

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-verify"
    Service = "worker-verify"
  })
}

resource "aws_ecr_lifecycle_policy" "worker_verify" {
  repository = aws_ecr_repository.worker_verify.name

  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep last ${var.retention_count} images"
      selection = {
        tagStatus   = "any"
        countType   = "imageCountMoreThan"
        countNumber = var.retention_count
      }
      action = {
        type = "expire"
      }
    }]
  })
}

# ECR Repository for Dashboard
resource "aws_ecr_repository" "dashboard" {
  name                 = "eyedar-${var.env_name}-dashboard"
  image_tag_mutability = var.image_tag_mutability

  image_scanning_configuration {
    scan_on_push = var.scan_on_push
  }

  encryption_configuration {
    encryption_type = "AES256"
  }

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-dashboard"
    Service = "dashboard"
  })
}

resource "aws_ecr_lifecycle_policy" "dashboard" {
  repository = aws_ecr_repository.dashboard.name

  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep last ${var.retention_count} images"
      selection = {
        tagStatus   = "any"
        countType   = "imageCountMoreThan"
        countNumber = var.retention_count
      }
      action = {
        type = "expire"
      }
    }]
  })
}

# ECR Repository for Worker-Notify
resource "aws_ecr_repository" "worker_notify" {
  name                 = "eyedar-${var.env_name}-worker-notify"
  image_tag_mutability = var.image_tag_mutability

  image_scanning_configuration {
    scan_on_push = var.scan_on_push
  }

  encryption_configuration {
    encryption_type = "AES256"
  }

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-worker-notify"
    Service = "worker-notify"
  })
}

resource "aws_ecr_lifecycle_policy" "worker_notify" {
  repository = aws_ecr_repository.worker_notify.name

  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep last ${var.retention_count} images"
      selection = {
        tagStatus   = "any"
        countType   = "imageCountMoreThan"
        countNumber = var.retention_count
      }
      action = {
        type = "expire"
      }
    }]
  })
}
