# WAF Web ACL
resource "aws_wafv2_web_acl" "main" {
  name  = "eyedar-${var.env_name}-waf"
  scope = "REGIONAL"

  default_action {
    allow {}
  }

  # Rule 1: AWS Managed Rules - Common Rule Set
  rule {
    name     = "AWSManagedRulesCommonRuleSet"
    priority = 1

    override_action {
      none {}
    }

    statement {
      managed_rule_group_statement {
        vendor_name = "AWS"
        name        = "AWSManagedRulesCommonRuleSet"
      }
    }

    visibility_config {
      cloudwatch_metrics_enabled = true
      metric_name                = "AWSManagedRulesCommonRuleSetMetric"
      sampled_requests_enabled   = true
    }
  }

  # Rule 2: AWS Managed Rules - Known Bad Inputs
  rule {
    name     = "AWSManagedRulesKnownBadInputsRuleSet"
    priority = 2

    override_action {
      none {}
    }

    statement {
      managed_rule_group_statement {
        vendor_name = "AWS"
        name        = "AWSManagedRulesKnownBadInputsRuleSet"
      }
    }

    visibility_config {
      cloudwatch_metrics_enabled = true
      metric_name                = "AWSManagedRulesKnownBadInputsRuleSetMetric"
      sampled_requests_enabled   = true
    }
  }

  # Rule 3: Rate Limiting
  dynamic "rule" {
    for_each = var.enable_rate_limiting ? [1] : []
    content {
      name     = "RateLimitRule"
      priority = 3

      action {
        block {}
      }

      statement {
        rate_based_statement {
          limit              = var.rate_limit
          aggregate_key_type = "IP"
        }
      }

      visibility_config {
        cloudwatch_metrics_enabled = true
        metric_name                = "RateLimitRuleMetric"
        sampled_requests_enabled   = true
      }
    }
  }

  # Rule 4: Block requests from specific countries (optional - add if needed)
  # Uncomment to enable geo-blocking
  # rule {
  #   name     = "GeoBlockingRule"
  #   priority = 4
  #
  #   action {
  #     block {}
  #   }
  #
  #   statement {
  #     geo_match_statement {
  #       country_codes = ["CN", "RU", "KP"]  # Example: block from China, Russia, North Korea
  #     }
  #   }
  #
  #   visibility_config {
  #     cloudwatch_metrics_enabled = true
  #     metric_name                = "GeoBlockingRuleMetric"
  #     sampled_requests_enabled   = true
  #   }
  # }

  visibility_config {
    cloudwatch_metrics_enabled = true
    metric_name                = "eyedar-${var.env_name}-waf"
    sampled_requests_enabled   = true
  }

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-waf"
  })
}

# Associate WAF with ALB
resource "aws_wafv2_web_acl_association" "main" {
  resource_arn = var.alb_arn
  web_acl_arn  = aws_wafv2_web_acl.main.arn
}
