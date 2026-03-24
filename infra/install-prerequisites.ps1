# Quick Install Script for Prerequisites
# Run this first if Terraform and AWS CLI are not installed

Write-Host "Installing Prerequisites for EyeDAR Infrastructure" -ForegroundColor Cyan
Write-Host ""

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "This script requires Administrator privileges." -ForegroundColor Yellow
    Write-Host "Please run PowerShell as Administrator and try again." -ForegroundColor Yellow
    exit 1
}

# Check if Chocolatey is installed
Write-Host "Checking for Chocolatey..." -ForegroundColor Yellow

if (-not (Get-Command choco -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Chocolatey..." -ForegroundColor Yellow
    Set-ExecutionPolicy Bypass -Scope Process -Force
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
    Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
    
    # Refresh PATH
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
}

Write-Host "✓ Chocolatey is installed" -ForegroundColor Green

# Install Terraform
Write-Host ""
Write-Host "Installing Terraform..." -ForegroundColor Yellow
choco install terraform -y

# Install AWS CLI
Write-Host ""
Write-Host "Installing AWS CLI..." -ForegroundColor Yellow
choco install awscli -y

# Refresh PATH
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "Installation Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Close and reopen PowerShell" -ForegroundColor White
Write-Host "2. Run: aws configure" -ForegroundColor White
Write-Host "3. Configure your AWS credentials" -ForegroundColor White
Write-Host "4. Run: .\deploy.ps1 -ValidateOnly" -ForegroundColor White
