param(
  [string]$Region = "us-east-1",
  [string]$Bucket = "eyedar-prod-objects-v2",
  [string]$QueueUrl = "https://sqs.us-east-1.amazonaws.com/937115287175/eyedar-prod-detection-created",
  [int]$Count = 1000,
  [int]$DeviceId = 1001,
  [string]$Severity = "medium",
  [string]$SensorSource = "camera",
  [switch]$DryRun
)

$ErrorActionPreference = "Stop"

if ($Count -le 0) {
  throw "Count must be > 0"
}

Write-Host "[INFO] Region: $Region"
Write-Host "[INFO] Bucket: $Bucket"
Write-Host "[INFO] Queue:  $QueueUrl"
Write-Host "[INFO] Count:  $Count"

# Validate prerequisites up front.
aws s3api head-bucket --bucket $Bucket --region $Region | Out-Null
if ($LASTEXITCODE -ne 0) {
  throw "S3 bucket '$Bucket' does not exist or is not accessible in region '$Region'."
}

aws sqs get-queue-attributes --queue-url $QueueUrl --attribute-names QueueArn --region $Region | Out-Null
if ($LASTEXITCODE -ne 0) {
  throw "SQS queue '$QueueUrl' does not exist or is not accessible in region '$Region'."
}

# Create one synthetic 640x640 image and reuse it for all events.
$tempImage = Join-Path $env:TEMP "eyedar-loadtest-640x640.jpg"
Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap 640, 640
$graphics = [System.Drawing.Graphics]::FromImage($bmp)
$graphics.Clear([System.Drawing.Color]::FromArgb(35, 40, 55))
$font = New-Object System.Drawing.Font("Arial", 26)
$brush = [System.Drawing.Brushes]::White
$graphics.DrawString("EYEDAR LOAD TEST", $font, $brush, 100, 280)
$graphics.Dispose()
$bmp.Save($tempImage, [System.Drawing.Imaging.ImageFormat]::Jpeg)
$bmp.Dispose()

$objectKey = "loadtest/640x640/base-image-$(Get-Date -Format 'yyyyMMdd-HHmmss').jpg"
$s3Uri = "s3://$Bucket/$objectKey"

if (-not $DryRun) {
  aws s3 cp $tempImage $s3Uri --region $Region | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to upload base image to $s3Uri"
  }
  Write-Host "[OK] Uploaded base image: $s3Uri"
} else {
  Write-Host "[DRYRUN] Would upload base image: $s3Uri"
}

Remove-Item -Force $tempImage

$sent = 0
$batchSize = 10
$start = Get-Date

for ($batchStart = 0; $batchStart -lt $Count; $batchStart += $batchSize) {
  $entries = @()

  for ($i = 0; $i -lt $batchSize; $i++) {
    $index = $batchStart + $i
    if ($index -ge $Count) { break }

    $eventTimestamp = (Get-Date).ToUniversalTime().ToString("o")
    $event = @{
      device_id = $DeviceId
      message = "Load test detection #$index"
      severity = $Severity
      sensor_source = $SensorSource
      timestamp = $eventTimestamp
      images = @($s3Uri)
    } | ConvertTo-Json -Compress

    $entry = [ordered]@{
      Id = "msg-$index"
      MessageBody = $event
    }

    $entries += $entry
  }

  if ($entries.Count -eq 0) { continue }

  if (-not $DryRun) {
    $requestJsonFile = Join-Path $env:TEMP "eyedar-sqs-request-$batchStart.json"
    @{
      QueueUrl = $QueueUrl
      Entries  = $entries
    } | ConvertTo-Json -Depth 5 -Compress | Set-Content -Path $requestJsonFile -Encoding ASCII

    $result = aws sqs send-message-batch --cli-input-json file://$requestJsonFile --region $Region | ConvertFrom-Json
    Remove-Item -Force $requestJsonFile

    if ($LASTEXITCODE -ne 0) {
      throw "send-message-batch failed at batch start index $batchStart"
    }

    $okCount = if ($null -ne $result.Successful) { $result.Successful.Count } else { 0 }
    $failCount = if ($null -ne $result.Failed) { $result.Failed.Count } else { 0 }

    if ($failCount -gt 0) {
      Write-Host "[WARN] Batch had failures: $failCount"
      $result.Failed | ConvertTo-Json -Depth 4
    }

    $sent += $okCount
  } else {
    $sent += $entries.Count
  }

  if (($batchStart + $batchSize) % 100 -eq 0 -or ($batchStart + $batchSize) -ge $Count) {
    Write-Host "[INFO] Progress: $sent/$Count"
  }
}

$elapsed = (Get-Date) - $start
Write-Host "[DONE] Events queued: $sent/$Count in $([Math]::Round($elapsed.TotalSeconds,2))s"
Write-Host "[NEXT] Check queue depth and worker logs:"
Write-Host "  aws sqs get-queue-attributes --queue-url $QueueUrl --attribute-names ApproximateNumberOfMessages ApproximateNumberOfMessagesNotVisible --region $Region"
Write-Host "  aws logs tail /ecs/eyedar-prod-worker-ingest --follow --region $Region"
