# Alert Routes AWS Integration Changes

## Changes needed in src/routes/alerts.rs

### 1. Update `save_base64_image` calls (Lines 246-250)

**BEFORE:**
```rust
let saved = save_base64_image(
    single_b64.clone(),
    detection.image_format.clone(),
    device_id
).await?;
```

**AFTER:**
```rust
let saved = save_base64_image(
    single_b64.clone(),
    detection.image_format.clone(),
    device_id,
    &s3_client,               // Add S3 client from app state
    &s3_bucket                // Add bucket name from app state
).await?;
```

### 2. Update `save_raw_image` calls (Lines 276-280)

**BEFORE:**
```rust
save_raw_image(
    raw_bytes.clone(),
    per_image_format,
    device_id
).await?
```

**AFTER:**
```rust
save_raw_image(
    raw_bytes.clone(),
    per_image_format,
    device_id,
    &s3_client,               // Add S3 client
    &s3_bucket                // Add bucket name
).await?
```

### 3. Update function signatures

**BEFORE:**
```rust
async fn save_base64_image(
    base64_data: String,
    image_format: Option<String>,
    device_id: u32,
) -> Result<Option<String>, Error> {
    let filename = ImageService::save_base64_image(base64_data, image_format, device_id).await?;
    Ok(Some(filename))
}

async fn save_raw_image(
    image_bytes: Vec<u8>,
    image_format: Option<String>,
    device_id: u32,
) -> Result<Option<String>, Error> {
    let filename = ImageService::save_raw_image(image_bytes, image_format, device_id).await?;
    Ok(Some(filename))
}
```

**AFTER:**
```rust
async fn save_base64_image(
    base64_data: String,
    image_format: Option<String>,
    device_id: u32,
    s3_client: &aws_sdk_s3::Client,
    s3_bucket: &str,
) -> Result<Option<String>, Error> {
    let s3_uri = ImageService::save_base64_image(
        base64_data,
        image_format,
        device_id,
        s3_client,
        s3_bucket
    ).await?;
    Ok(Some(s3_uri))
}

async fn save_raw_image(
    image_bytes: Vec<u8>,
    image_format: Option<String>,
    device_id: u32,
    s3_client: &aws_sdk_s3::Client,
    s3_bucket: &str,
) -> Result<Option<String>, Error> {
    let s3_uri = ImageService::save_raw_image(
        image_bytes,
        image_format,
        device_id,
        s3_client,
        s3_bucket
    ).await?;
    Ok(Some(s3_uri))
}
```

### 4. Add SQS publishing after image save (Line 345)

**BEFORE:**
```rust
return Ok(HttpResponse::Accepted().json(serde_json::json!({
    "status": "detection_received",
    "message": detection.message,
    "device_id": device_id,
    "timestamp": detection.timestamp,
    "image_saved": image_path.is_some(),
    "image_path": image_path,
})));
```

**AFTER:**
```rust
// Publish detection_created event to SQS for processing
if let Some(queue_url) = queue_url_ingest {
    let s3_images = if let Some(ref img_path) = image_path {
        vec![img_path.clone()]
    } else {
        vec![]
    };

    if let Err(e) = crate::services::sqs_service::SqsService::publish_detection_created(
        &sqs_client,
        &queue_url,
        device_id,
        s3_images,
        &detection.message,
        &detection.timestamp,
        detection.severity.as_deref().unwrap_or("unknown"),
        detection.sensor_source.as_deref(),
    ).await {
        eprintln!("⚠️  Failed to publish to SQS: {}", e);
        // Don't fail the request, just log it
    }
}

return Ok(HttpResponse::Accepted().json(serde_json::json!({
    "status": "detection_received",
    "message": detection.message,
    "device_id": device_id,
    "timestamp": detection.timestamp,
    "image_saved": image_path.is_some(),
    "image_path": image_path,
})));
```

### 5. Update `post_alert` handler signature

**BEFORE:**
```rust
#[post("/alerts")]
pub async fn post_alert(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
```

**AFTER:**
```rust
#[post("/alerts")]
pub async fn post_alert(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
    s3_client: web::Data<aws_sdk_s3::Client>,
    s3_bucket: web::Data<String>,
    sqs_client: web::Data<aws_sdk_sqs::Client>,
    queue_url_ingest: web::Data<String>,
) -> Result<HttpResponse, Error> {
```

### 6. Update `handle_json_alert` handler signature

**BEFORE:**
```rust
async fn handle_json_alert(
    _req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
```

**AFTER:**
```rust
async fn handle_json_alert(
    _req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<PgPool>,
    s3_client: &aws_sdk_s3::Client,
    s3_bucket: &str,
    sqs_client: &aws_sdk_sqs::Client,
    queue_url_ingest: &str,
) -> Result<HttpResponse, Error> {
```

And pass these through to the image save calls:
```rust
let saved = save_base64_image(
    single_b64.clone(),
    detection.image_format.clone(),
    device_id,
    s3_client,
    s3_bucket
).await?;
```

### 7. Add imports at top of file

```rust
use crate::services::sqs_service::SqsService;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_sqs::Client as SqsClient;
```

### 8. Update `post_device_health` handler

**BEFORE:**
```rust
#[post("/device/health")]
pub async fn post_device_health(
    body: web::Bytes,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
```

**AFTER:**
```rust
#[post("/device/health")]
pub async fn post_device_health(
    body: web::Bytes,
    pool: web::Data<PgPool>,
    // These can be optional since device health doesn't save images
) -> Result<HttpResponse, Error> {
```

## Summary of Changes

1. **S3 Client & Bucket**: Passed from main.rs app state
2. **SQS Client & URL**: Passed from main.rs app state
3. **Image saving**: Now returns S3 URIs (not local paths)
4. **SQS publishing**: After image save completes, publish detection_created message
5. **Function signatures**: Updated to accept AWS clients and buckets
6. **Env vars**: QUEUE_URL_INGEST set from Terraform output

All changes maintain backward compatibility and add logging to track AWS operations.
