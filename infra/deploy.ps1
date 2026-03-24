# Infrastructure Deployment Automation Script
# This script validates, installs prerequisites, and deploys the EyeDAR infrastructure

param(
    [switch]$SkipInstall = $false,
    [switch]$ValidateOnly = $false,
    [switch]$PlanOnly = $false,
    [switch]$Apply = $false
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "EyeDAR Infrastructure Deployment Agent" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Check prerequisites
Write-Host "[1/6] Checking prerequisites..." -ForegroundColor Yellow

# Check Terraform
$terraformInstalled = $false
try {
    $tfVersion = terraform version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Terraform is installed: $($tfVersion -split "`n" | Select-Object -First 1)" -ForegroundColor Green
        $terraformInstalled = $true
    }
} catch {
    Write-Host "✗ Terraform is not installed" -ForegroundColor Red
}

# Check AWS CLI
$awsInstalled = $false
try {
    $awsVersion = aws --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ AWS CLI is installed: $awsVersion" -ForegroundColor Green
        $awsInstalled = $true
    }
} catch {
    Write-Host "✗ AWS CLI is not installed" -ForegroundColor Red
}

# Step 2: Install missing prerequisites
if (-not $SkipInstall) {
    Write-Host ""
    Write-Host "[2/6] Installing missing prerequisites..." -ForegroundColor Yellow
    
    if (-not $terraformInstalled) {
        Write-Host "Installing Terraform..." -ForegroundColor Yellow
        
        # Check if Chocolatey is available
        if (Get-Command choco -ErrorAction SilentlyContinue) {
            choco install terraform -y
        } else {
            Write-Host "Please install Terraform manually from: https://www.terraform.io/downloads" -ForegroundColor Red
            Write-Host "Or install Chocolatey first: https://chocolatey.org/install" -ForegroundColor Red
            exit 1
        }
    }
    
    if (-not $awsInstalled) {
        Write-Host "Installing AWS CLI..." -ForegroundColor Yellow
        
        if (Get-Command choco -ErrorAction SilentlyContinue) {
            choco install awscli -y
        } else {
            Write-Host "Please install AWS CLI manually from: https://aws.amazon.com/cli/" -ForegroundColor Red
            exit 1
        }
    }
    
    # Refresh PATH
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
}

# Step 3: Validate AWS credentials
Write-Host ""
Write-Host "[3/6] Checking AWS credentials..." -ForegroundColor Yellow

try {
    $awsIdentity = aws sts get-caller-identity 2>&1
    if ($LASTEXITCODE -eq 0) {
        $identity = $awsIdentity | ConvertFrom-Json
        Write-Host "✓ AWS credentials are configured" -ForegroundColor Green
        Write-Host "  Account: $($identity.Account)" -ForegroundColor Gray
        Write-Host "  User/Role: $($identity.Arn)" -ForegroundColor Gray
    } else {
        Write-Host "✗ AWS credentials are not configured" -ForegroundColor Red
        Write-Host "  Please run: aws configure" -ForegroundColor Yellow
        exit 1
    }
} catch {
    Write-Host "✗ Failed to check AWS credentials" -ForegroundColor Red
    Write-Host "  Error: $_" -ForegroundColor Red
    exit 1
}

# Step 4: Validate Terraform configuration
Write-Host ""
Write-Host "[4/6] Validating Terraform configuration..." -ForegroundColor Yellow

$infraPath = Join-Path $PSScriptRoot "envs\prod"
Set-Location $infraPath

# Check if terraform.tfvars exists
if (-not (Test-Path "terraform.tfvars")) {
    Write-Host "✗ terraform.tfvars not found" -ForegroundColor Red
    Write-Host "  Please copy terraform.tfvars.example to terraform.tfvars and configure it" -ForegroundColor Yellow
    
    if (Test-Path "terraform.tfvars.example") {
        Write-Host "  Running: Copy-Item terraform.tfvars.example terraform.tfvars" -ForegroundColor Yellow
        Copy-Item terraform.tfvars.example terraform.tfvars
        Write-Host "✓ Created terraform.tfvars from example" -ForegroundColor Green
        Write-Host "⚠  Please edit terraform.tfvars with your actual values before continuing" -ForegroundColor Yellow
        exit 0
    } else {
        exit 1
    }
}

Write-Host "✓ terraform.tfvars found" -ForegroundColor Green

# Count Terraform files
$tfFiles = Get-ChildItem -Path "..\..\modules" -Filter "*.tf" -Recurse
Write-Host "✓ Found $($tfFiles.Count) Terraform module files" -ForegroundColor Green

# Step 5: Initialize Terraform
Write-Host ""
Write-Host "[5/6] Initializing Terraform..." -ForegroundColor Yellow

terraform init

if ($LASTEXITCODE -ne 0) {
    Write-Host "✗ Terraform init failed" -ForegroundColor Red
    exit 1
}

Write-Host "✓ Terraform initialized successfully" -ForegroundColor Green

# Validate syntax
Write-Host ""
Write-Host "Validating Terraform syntax..." -ForegroundColor Yellow
terraform validate

if ($LASTEXITCODE -ne 0) {
    Write-Host "✗ Terraform validation failed" -ForegroundColor Red
    exit 1
}

Write-Host "✓ Terraform configuration is valid" -ForegroundColor Green

if ($ValidateOnly) {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "Validation completed successfully!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    exit 0
}

# Step 6: Plan/Apply infrastructure
Write-Host ""
Write-Host "[6/6] Planning infrastructure deployment..." -ForegroundColor Yellow

terraform plan -out=tfplan

if ($LASTEXITCODE -ne 0) {
    Write-Host "✗ Terraform plan failed" -ForegroundColor Red
    exit 1
}

Write-Host "✓ Terraform plan completed successfully" -ForegroundColor Green

if ($PlanOnly) {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "Plan completed. Review the output above." -ForegroundColor Green
    Write-Host "To apply, run with -Apply flag" -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Green
    exit 0
}

if ($Apply) {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Magenta
    Write-Host "APPLYING INFRASTRUCTURE CHANGES" -ForegroundColor Magenta
    Write-Host "========================================" -ForegroundColor Magenta
    Write-Host ""
    Write-Host "This will create real AWS resources and incur costs!" -ForegroundColor Yellow
    Write-Host ""
    
    $confirmation = Read-Host "Type 'yes' to confirm deployment"
    
    if ($confirmation -eq "yes") {
        terraform apply tfplan
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Write-Host "========================================" -ForegroundColor Green
            Write-Host "✓ Infrastructure deployed successfully!" -ForegroundColor Green
            Write-Host "========================================" -ForegroundColor Green
            Write-Host ""
            Write-Host "Next steps:" -ForegroundColor Yellow
            Write-Host "1. Update Secrets Manager with real credentials" -ForegroundColor White
            Write-Host "2. Push Docker images to ECR" -ForegroundColor White
            Write-Host "3. Update ECS services to use the new images" -ForegroundColor White
        } else {
            Write-Host "✗ Infrastructure deployment failed" -ForegroundColor Red
            exit 1
        }
    } else {
        Write-Host "Deployment cancelled" -ForegroundColor Yellow
    }
} else {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "Ready to deploy!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "To deploy the infrastructure, run:" -ForegroundColor Yellow
    Write-Host "  .\deploy.ps1 -Apply" -ForegroundColor White
}
