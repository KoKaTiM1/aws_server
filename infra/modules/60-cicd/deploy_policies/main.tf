# GitHub Actions deployment policies
# Allows GitHub Actions OIDC role to push to ECR and update ECS services

# ========== ECR Push Permission ==========
resource "aws_iam_role_policy" "github_ecr_push" {
  name   = "github-ecr-push"
  role   = var.github_deploy_role_arn
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "ECRAuthGetToken"
        Effect = "Allow"
        Action = [
          "ecr:GetAuthorizationToken"
        ]
        Resource = "*"
      },
      {
        Sid    = "ECRPushImage"
        Effect = "Allow"
        Action = [
          "ecr:BatchCheckLayerAvailability",
          "ecr:CompleteLayerUpload",
          "ecr:InitiateLayerUpload",
          "ecr:PutImage",
          "ecr:UploadLayerPart",
          "ecr:DescribeRepositories",
          "ecr:DescribeImages"
        ]
        Resource = var.ecr_repo_arns
      }
    ]
  })
}

# ========== ECS Update Permission ==========
resource "aws_iam_role_policy" "github_ecs_update" {
  name   = "github-ecs-update"
  role   = var.github_deploy_role_arn
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "ECSUpdateService"
        Effect = "Allow"
        Action = [
          "ecs:UpdateService",
          "ecs:DescribeServices",
          "ecs:DescribeTaskDefinition",
          "ecs:DescribeCluster"
        ]
        Resource = [
          var.ecs_cluster_arn,
          "arn:aws:ecs:*:*:service/*/${var.ecs_cluster_name}/*"
        ]
      },
      {
        Sid    = "IAMPassRole"
        Effect = "Allow"
        Action = [
          "iam:PassRole"
        ]
        Resource = [
          "arn:aws:iam::*:role/ecsTaskExecutionRole*",
          "arn:aws:iam::*:role/ecsTaskRole*"
        ]
        Condition = {
          StringEquals = {
            "iam:PassedToService" = "ecs-tasks.amazonaws.com"
          }
        }
      }
    ]
  })
}

# ========== CloudWatch Logs Permission (for deployment monitoring) ==========
resource "aws_iam_role_policy" "github_logs" {
  name   = "github-logs-read"
  role   = var.github_deploy_role_arn
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "CloudWatchLogs"
        Effect = "Allow"
        Action = [
          "logs:DescribeLogGroups",
          "logs:DescribeLogStreams",
          "logs:GetLogEvents"
        ]
        Resource = [
          "arn:aws:logs:*:*:log-group:/ecs/*"
        ]
      }
    ]
  })
}
