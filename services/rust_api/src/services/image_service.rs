use actix_web::Error;
use base64::{Engine as _, engine::general_purpose};
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use std::error::Error as StdError;

pub struct ImageService;

impl ImageService {
    /// Save a base64 encoded image to S3
    pub async fn save_base64_image(
        base64_data: String,
        image_format: Option<String>,
        device_id: u32,
        s3_client: &S3Client,
        s3_bucket: &str,
    ) -> Result<String, Error> {
        println!("📸 ImageService::save_base64_image (S3) called for device {}", device_id);

        let original_data = base64_data.clone();

        // Clean up base64 data (remove data URL prefix if present)
        let clean_data = if base64_data.contains(',') {
            base64_data.split(",").last().unwrap_or(&base64_data).to_string()
        } else {
            base64_data
        };

        // Decode base64
        let image_bytes = general_purpose::STANDARD.decode(clean_data)
            .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid base64 data: {}", e)))?;

        // Determine file extension
        let extension = image_format.unwrap_or_else(|| {
            if original_data.contains("data:image/png") {
                "png".to_string()
            } else if original_data.contains("data:image/jpeg") || original_data.contains("data:image/jpg") {
                "jpg".to_string()
            } else if original_data.contains("data:image/webp") {
                "webp".to_string()
            } else {
                "jpg".to_string()
            }
        });

        // Create S3 object key: detections/{device_id}/{timestamp}_{uuid}.{format}
        let timestamp = chrono::Utc::now().timestamp_millis();
        let unique_id = uuid::Uuid::new_v4();
        let s3_key = format!("detections/{}/{}_{}.{}", device_id, timestamp, unique_id, extension);

        // Upload to S3
        let byte_stream = ByteStream::from(image_bytes);
        println!("📤 S3 Upload (base64): bucket={}, key={}, content-type={}",
                 s3_bucket, s3_key, Self::get_mime_type(&extension));

        let result = s3_client
            .put_object()
            .bucket(s3_bucket)
            .key(&s3_key)
            .body(byte_stream)
            .content_type(Self::get_mime_type(&extension))
            .send()
            .await;

        match result {
            Ok(_) => {
                let s3_uri = format!("s3://{}/{}", s3_bucket, s3_key);
                println!("✅ Base64 image uploaded to S3: {}", s3_uri);
                return Ok(s3_uri);
            },
            Err(e) => {
                eprintln!("❌ S3 upload error: {}", e);
                eprintln!("   Full error: {:?}", e);
                return Err(actix_web::error::ErrorInternalServerError(
                    format!("S3 put_object failed: {}", e)
                ));
            }
        }
    }

    /// Save raw image bytes to S3
    pub async fn save_raw_image(
        image_bytes: Vec<u8>,
        image_format: Option<String>,
        device_id: u32,
        s3_client: &S3Client,
        s3_bucket: &str,
    ) -> Result<String, Error> {
        println!("📸 ImageService::save_raw_image (S3) called for device {}", device_id);

        if image_bytes.is_empty() {
            return Err(actix_web::error::ErrorBadRequest("Empty image data"));
        }

        let extension = image_format.unwrap_or_else(|| "jpg".to_string());

        // Create S3 object key: detections/{device_id}/{timestamp}_{uuid}.{format}
        let timestamp = chrono::Utc::now().timestamp_millis();
        let unique_id = uuid::Uuid::new_v4();
        let s3_key = format!("detections/{}/{}_{}.{}", device_id, timestamp, unique_id, extension);

        // Upload to S3
        let byte_stream = ByteStream::from(image_bytes);
        println!("📤 S3 Upload (raw): bucket={}, key={}, content-type={}",
                 s3_bucket, s3_key, Self::get_mime_type(&extension));

        let result = s3_client
            .put_object()
            .bucket(s3_bucket)
            .key(&s3_key)
            .body(byte_stream)
            .content_type(Self::get_mime_type(&extension))
            .send()
            .await;

        match result {
            Ok(_) => {
                let s3_uri = format!("s3://{}/{}", s3_bucket, s3_key);
                println!("✅ Raw image uploaded to S3: {}", s3_uri);
                return Ok(s3_uri);
            },
            Err(e) => {
                eprintln!("❌ S3 raw upload error: {}", e);
                eprintln!("   Full error: {:?}", e);
                return Err(actix_web::error::ErrorInternalServerError(
                    format!("S3 raw image upload failed: {}", e)
                ));
            }
        }
    }

    /// Validate image format
    pub fn validate_image_format(format: &str) -> Result<(), String> {
        match format.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "webp" | "gif" => Ok(()),
            _ => Err(format!("Unsupported image format: {}", format)),
        }
    }

    /// Get the MIME type for an image format
    pub fn get_mime_type(format: &str) -> &'static str {
        match format.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            _ => "application/octet-stream",
        }
    }

    /// Validate base64 image size (prevent huge uploads)
    pub fn validate_base64_size(base64_data: &str, max_size_mb: f64) -> Result<(), String> {
        let estimated_bytes = (base64_data.len() as f64 * 0.75) as u64;
        let max_bytes = (max_size_mb * 1024.0 * 1024.0) as u64;

        if estimated_bytes > max_bytes {
            return Err(format!(
                "Image too large: {:.1}MB (max: {:.1}MB)",
                estimated_bytes as f64 / 1024.0 / 1024.0,
                max_size_mb
            ));
        }
        Ok(())
    }

    /// Validate raw image size (prevent huge uploads)
    pub fn validate_raw_image_size(image_bytes: &[u8], max_size_mb: f64) -> Result<(), String> {
        let size_bytes = image_bytes.len() as u64;
        let max_bytes = (max_size_mb * 1024.0 * 1024.0) as u64;

        if size_bytes > max_bytes {
            return Err(format!(
                "Image too large: {:.1}MB (max: {:.1}MB)",
                size_bytes as f64 / 1024.0 / 1024.0,
                max_size_mb
            ));
        }
        Ok(())
    }
}
