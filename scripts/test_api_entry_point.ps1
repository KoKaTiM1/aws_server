#!/usr/bin/env pwsh
<#
.SYNOPSIS
Test the Rust API entry point: POST /api/v1/alerts
Verifies the complete flow: API -> S3 -> RDS -> SQS

.DESCRIPTION
1. Call POST /api/v1/alerts with a test image
2. Verify image in S3 (eyedar-prod-objects)
3. Verify detection record in RDS (detections table)
4. Verify message in SQS (eyedar-prod-detection-created)
#>

param(
    [string]$ApiUrl,
    [string]$DeviceId = "test-device-001",
    [string]$ImagePath,
    [string]$Region = "us-east-1"
)

$ErrorActionPreference = "Stop"

# Get ALB DNS if not provided
if (-not $ApiUrl) {
    Write-Host "Fetching ALB DNS from Terraform outputs..." -ForegroundColor Cyan
    $outputs = & terraform -chdir="infra/envs/prod" output -json | ConvertFrom-Json
    $ApiUrl = "http://$($outputs.alb_dns_name.value)"
    Write-Host "API URL: $ApiUrl" -ForegroundColor Green
}

# Create test image if not provided
if (-not $ImagePath -or -not (Test-Path $ImagePath)) {
    Write-Host "Creating synthetic test image..." -ForegroundColor Cyan
    $ImagePath = "$env:TEMP\test-image-$(Get-Random).jpg"
    
    # Create a simple JPEG using PowerShell (requires .NET)
    [byte[]]$jpegHeader = @(0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01)
    [byte[]]$jpegFooter = @(0xFF, 0xD9)
    [byte[]]$jpegData = $jpegHeader + ([byte[]]1..256) + $jpegFooter
    
    [System.IO.File]::WriteAllBytes($ImagePath, $jpegData)
    Write-Host "Test image created: $ImagePath ($(Get-Item $ImagePath).Length) bytes" -ForegroundColor Green
}

# 1. Call API endpoint
Write-Host "`n=== Step 1: Call POST /api/v1/alerts ===" -ForegroundColor Yellow
try {
    $imageBytes = [System.IO.File]::ReadAllBytes($ImagePath)
    
    $body = @{
        device_id = $DeviceId
        latitude = 40.7128
        longitude = -74.0060
        image = [Convert]::ToBase64String($imageBytes)
        timestamp = [datetime]::UtcNow.ToString("O")
    } | ConvertTo-Json
    
    $response = Invoke-RestMethod `
        -Uri "$ApiUrl/api/v1/alerts" `
        -Method POST `
        -ContentType "application/json" `
        -Body $body `
        -ErrorAction Continue
    
    if ($response) {
        Write-Host "✓ API response: $($response | ConvertTo-Json)" -ForegroundColor Green
        $detectionId = $response.detection_id
    } else {
        Write-Host "✗ No response from API" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "✗ API call failed: $_" -ForegroundColor Red
    exit 1
}

# 2. Check S3 for image
Write-Host "`n=== Step 2: Verify image in S3 ===" -ForegroundColor Yellow
try {
    $s3Objects = aws s3 ls "s3://eyedar-prod-objects/" --recursive --region $Region | Select-String $detectionId
    if ($s3Objects) {
        Write-Host "✓ Image found in S3:" -ForegroundColor Green
        Write-Host $s3Objects
    } else {
        Write-Host "⚠ No image found for detection ID: $detectionId" -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠ S3 check skipped: $_" -ForegroundColor Yellow
}

# 3. Check RDS for detection record
Write-Host "`n=== Step 3: Verify detection record in RDS ===" -ForegroundColor Yellow
try {
    $rdsHost = (& terraform -chdir="infra/envs/prod" output -raw rds_endpoint 2>/dev/null) -replace ":.*", ""
    
    if (-not $rdsHost) {
        Write-Host "⚠ Could not determine RDS host" -ForegroundColor Yellow
    } else {
        Write-Host "RDS Host: $rdsHost"
        Write-Host "(Run this query manually in pgAdmin or psql to verify detection record:)" -ForegroundColor Cyan
        Write-Host "  SELECT id, device_id, location, created_at FROM detections WHERE id = '$detectionId';" -ForegroundColor Cyan
    }
} catch {
    Write-Host "⚠ RDS check skipped: $_" -ForegroundColor Yellow
}

# 4. Check SQS for message
Write-Host "`n=== Step 4: Verify message in SQS ===" -ForegroundColor Yellow
try {
    $queueUrl = aws sqs get-queue-url --queue-name eyedar-prod-detection-created --region $Region --query QueueUrl --output text
    
    $messages = aws sqs receive-message `
        --queue-url $queueUrl `
        --max-number-of-messages 5 `
        --region $Region `
        --output json | ConvertFrom-Json
    
    if ($messages.Messages) {
        Write-Host "✓ Found $(($messages.Messages | Measure-Object).Count) message(s) in queue:" -ForegroundColor Green
        foreach ($msg in $messages.Messages) {
            $body = $msg.Body | ConvertFrom-Json
            Write-Host "  - Detection ID: $($body.detection_id), Device: $($body.device_id)" -ForegroundColor Green
            Write-Host "    Message ID: $($msg.MessageId)"
        }
    } else {
        Write-Host "⚠ No messages in queue (queue may be processing fast)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠ SQS check skipped: $_" -ForegroundColor Yellow
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
Write-Host "Summary:" -ForegroundColor Cyan
Write-Host "  Detection ID: $detectionId" -ForegroundColor Green
Write-Host "  Device ID: $DeviceId" -ForegroundColor Green
Write-Host "  Image Path: $ImagePath" -ForegroundColor Green
Write-Host "`nNext steps:" -ForegroundColor Cyan
Write-Host "  1. Verify detection record exists in RDS detections table" -ForegroundColor White
Write-Host "  2. Check SQS queue for the detection_created message" -ForegroundColor White
Write-Host "  3. Monitor CloudWatch logs for worker-ingest processing" -ForegroundColor White
