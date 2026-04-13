# Generate random password for initial RDS setup
# (will be replaced with Secrets Manager value after initial apply)
resource "random_password" "db" {
  length              = 32
  special             = true
  override_special    = "!#$%&*()-_=+[]{}<>:?"
}

# DB Subnet Group
resource "aws_db_subnet_group" "main" {
  name       = "eyedar-${var.env_name}-db-subnet-group"
  subnet_ids = var.private_subnet_ids

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-db-subnet-group"
  })
}

# Parameter Group
resource "aws_db_parameter_group" "main" {
  name   = "eyedar-${var.env_name}-postgres-params"
  family = "postgres15"

  parameter {
    name  = "log_connections"
    value = "1"
  }

  parameter {
    name  = "log_disconnections"
    value = "1"
  }

  parameter {
    name  = "log_duration"
    value = "1"
  }

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-postgres-params"
  })
}

# RDS PostgreSQL Instance
resource "aws_db_instance" "main" {
  identifier = "eyedar-${var.env_name}-db"

  engine         = "postgres"
  engine_version = "15.7"
  instance_class = var.instance_class

  allocated_storage     = var.allocated_storage
  max_allocated_storage = var.max_allocated_storage
  storage_type          = "gp3"
  storage_encrypted     = true
  kms_key_id            = var.kms_key_arn

  db_name  = "eyedar"
  username = "eyedar_admin"
  password = random_password.db.result

  db_subnet_group_name   = aws_db_subnet_group.main.name
  vpc_security_group_ids = [var.security_group_id]
  parameter_group_name   = aws_db_parameter_group.main.name

  multi_az               = var.multi_az
  publicly_accessible    = false
  deletion_protection    = var.deletion_protection
  skip_final_snapshot    = var.backup_retention_days == 0 ? true : false
  final_snapshot_identifier = var.backup_retention_days == 0 ? null : "eyedar-${var.env_name}-final-snapshot-${formatdate("YYYY-MM-DD-hhmm", timestamp())}"

  backup_retention_period = var.backup_retention_days
  backup_window          = var.backup_retention_days > 0 ? "03:00-04:00" : null
  maintenance_window     = "mon:04:00-mon:05:00"

  enabled_cloudwatch_logs_exports = ["postgresql", "upgrade"]

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-db"
  })

  lifecycle {
    ignore_changes = [
      password,
      final_snapshot_identifier
    ]
  }
}

