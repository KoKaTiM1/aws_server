# Application Load Balancer
resource "aws_lb" "main" {
  name               = "eyedar-${var.env_name}-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [var.security_group_id]
  subnets            = var.public_subnet_ids

  enable_deletion_protection = false
  enable_http2              = true
  enable_cross_zone_load_balancing = true

  dynamic "access_logs" {
    for_each = var.enable_access_logs ? [1] : []
    content {
      bucket  = var.access_logs_bucket
      enabled = true
    }
  }

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-alb"
  })
}

# Target Group for API
resource "aws_lb_target_group" "api" {
  name        = "eyedar-${var.env_name}-api-tg"
  port        = var.api_target_config.port
  protocol    = "HTTP"
  vpc_id      = var.vpc_id
  target_type = "ip"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 3
    timeout             = 5
    interval            = 30
    path                = "/health"
    matcher             = "200"
  }

  deregistration_delay = 30

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-api-tg"
    Service = "api"
  })
}

# Target Group for Dashboard
resource "aws_lb_target_group" "dashboard" {
  name        = "eyedar-${var.env_name}-dashboard-tg"
  port        = var.dashboard_target_config.port
  protocol    = "HTTP"
  vpc_id      = var.vpc_id
  target_type = "ip"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 3
    timeout             = 5
    interval            = 30
    path                = "/health"
    matcher             = "200"
  }

  deregistration_delay = 30

  tags = merge(var.tags, {
    Name    = "eyedar-${var.env_name}-dashboard-tg"
    Service = "dashboard"
  })
}

# HTTP Listener (direct routing - no HTTPS for now)
resource "aws_lb_listener" "http" {
  load_balancer_arn = aws_lb.main.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type = "fixed-response"

    fixed_response {
      content_type = "text/plain"
      message_body = "Not Found"
      status_code  = "404"
    }
  }
}

# Listener Rule for API (path: /api/*)
resource "aws_lb_listener_rule" "api" {
  listener_arn = aws_lb_listener.http.arn
  priority     = 100

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.api.arn
  }

  condition {
    path_pattern {
      values = ["/api/*"]
    }
  }
}

# Listener Rule for Dashboard (path: /dashboard/*)
resource "aws_lb_listener_rule" "dashboard" {
  listener_arn = aws_lb_listener.http.arn
  priority     = 200

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.dashboard.arn
  }

  condition {
    path_pattern {
      values = ["/dashboard/*"]
    }
  }
}

# Also route root to dashboard
resource "aws_lb_listener_rule" "dashboard_root" {
  listener_arn = aws_lb_listener.http.arn
  priority     = 300

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.dashboard.arn
  }

  condition {
    path_pattern {
      values = ["/"]
    }
  }
}
