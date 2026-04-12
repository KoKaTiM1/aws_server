use actix_web::Error;
use base64::{Engine as _, engine::general_purpose};
use tokio::fs;
use std::os::unix::fs::PermissionsExt;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;

pub struct ImageService;

impl ImageService {
    /// Save a base64 encoded image to the filesystem organized by device_id
    /// Also creates a copy in 'latest' folder for dashboard display (non-blocking)
    pub async fn save_base64_image(
        base64_data: String,
        image_format: Option<String>,
        device_id: u32,
    ) -> Result<String, Error> {
        println!("📸 ImageService::save_base64_image called for device {}", device_id);
        // Store original data for format detection before moving
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
            // Try to detect format from original data or default to jpg
            if original_data.contains("data:image/png") {
                "png".to_string()
            } else if original_data.contains("data:image/jpeg") || original_data.contains("data:image/jpg") {
                "jpg".to_string()
            } else if original_data.contains("data:image/webp") {
                "webp".to_string()
            } else {
                "jpg".to_string() // Default
            }
        });

        // Create unique filename with timestamp
        let timestamp = chrono::Utc::now().timestamp_millis();
        let unique_id = uuid::Uuid::new_v4();
        let filename = format!("alert_image_{}_{}.{}", timestamp, unique_id, extension);

        // Save to serengeti/esp_photos/device_id directory
        let device_photos_dir = format!("./serengeti/esp_photos/{}", device_id);
        
        // Create directory if it doesn't exist
        fs::create_dir_all(&device_photos_dir).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to create device photos directory: {}", e)
            ))?;

        // Set directory permissions to 777 (rwxrwxrwx) - full access
        // This ensures all processes can read, write, execute, and delete files
        let dir_metadata = fs::metadata(&device_photos_dir).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to read directory metadata: {}", e)
            ))?;
        let mut dir_permissions = dir_metadata.permissions();
        dir_permissions.set_mode(0o777);
        fs::set_permissions(&device_photos_dir, dir_permissions).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to set directory permissions: {}", e)
            ))?;

        let file_path = format!("{}/{}", device_photos_dir, filename);
        println!("✅ Writing image to: {}", file_path);
        
        // Write file to disk (PRIMARY - for inference watcher)
        fs::write(&file_path, &image_bytes).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to save image: {}", e)
            ))?;

        // Set file permissions to 666 (rw-rw-rw-)
        // All users can read/write (needed for inference watcher and cleanup)
        let file_metadata = fs::metadata(&file_path).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to read file metadata: {}", e)
            ))?;
        let mut file_permissions = file_metadata.permissions();
        file_permissions.set_mode(0o666);
        fs::set_permissions(&file_path, file_permissions).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to set file permissions: {}", e)
            ))?;

        // Create copy in 'latest' folder for dashboard (non-blocking, best effort)
        // This copy persists for dashboard viewing even after inference watcher moves the original
        let latest_dir = format!("{}/latest", device_photos_dir);
        let latest_filename = format!("latest.{}", extension);
        let image_bytes_clone = image_bytes.clone();
        
        // Spawn async task so it doesn't block the inference watcher's file processing
        tokio::spawn(async move {
            if let Err(e) = Self::save_latest_copy(latest_dir, latest_filename, image_bytes_clone).await {
                eprintln!("⚠️  Failed to save latest copy (non-critical): {}", e);
            }
        });

        Ok(file_path) // Return full file path for storage
    }

    /// Save image to MinIO (optional - for cloud backup and retrieval)
    /// This runs asynchronously after filesystem save completes
    pub async fn save_to_minio(
        s3_client: &S3Client,
        bucket: &str,
        object_key: &str,
        image_bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let byte_stream = ByteStream::from(image_bytes);
        
        s3_client
            .put_object()
            .bucket(bucket)
            .key(object_key)
            .body(byte_stream)
            .content_type(content_type)
            .send()
            .await?;
        
        let minio_url = format!("s3://{}/{}", bucket, object_key);
        println!("☁️  Image uploaded to MinIO: {}", minio_url);
        Ok(minio_url)
    }

    /// Save a copy to the 'latest' folder (separate async task, non-blocking)
    /// This ensures dashboard always has a photo to display
    async fn save_latest_copy(
        latest_dir: String,
        latest_filename: String,
        image_bytes: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create latest directory
        fs::create_dir_all(&latest_dir).await?;

        // Set directory permissions
        let dir_metadata = fs::metadata(&latest_dir).await?;
        let mut dir_permissions = dir_metadata.permissions();
        dir_permissions.set_mode(0o777);
        fs::set_permissions(&latest_dir, dir_permissions).await?;

        let latest_path = format!("{}/{}", latest_dir, latest_filename);

        // Write latest copy (this overwrites previous latest)
        fs::write(&latest_path, &image_bytes).await?;

        // Set file permissions
        let file_metadata = fs::metadata(&latest_path).await?;
        let mut file_permissions = file_metadata.permissions();
        file_permissions.set_mode(0o666);
        fs::set_permissions(&latest_path, file_permissions).await?;

        println!("📋 Dashboard copy saved: {}", latest_path);
        Ok(())
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
        let estimated_bytes = (base64_data.len() as f64 * 0.75) as u64; // Base64 is ~33% larger
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

    /// Save raw image bytes to the filesystem organized by device_id
    /// Also creates a copy in 'latest' folder for dashboard display (non-blocking)
    pub async fn save_raw_image(
        image_bytes: Vec<u8>,
        image_format: Option<String>,
        device_id: u32,
    ) -> Result<String, Error> {
        println!("📸 ImageService::save_raw_image called for device {}", device_id);
        
        if image_bytes.is_empty() {
            return Err(actix_web::error::ErrorBadRequest("Empty image data"));
        }

        // Determine file extension
        let extension = image_format.unwrap_or_else(|| "jpg".to_string());

        // Create unique filename with timestamp
        let timestamp = chrono::Utc::now().timestamp_millis();
        let unique_id = uuid::Uuid::new_v4();
        let filename = format!("alert_image_{}_{}.{}", timestamp, unique_id, extension);

        // Save to serengeti/esp_photos/device_id directory
        let device_photos_dir = format!("./serengeti/esp_photos/{}", device_id);
        
        // Create directory if it doesn't exist
        fs::create_dir_all(&device_photos_dir).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to create device photos directory: {}", e)
            ))?;

        // Set directory permissions to 777 (rwxrwxrwx) - full access
        let dir_metadata = fs::metadata(&device_photos_dir).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to read directory metadata: {}", e)
            ))?;
        let mut dir_permissions = dir_metadata.permissions();
        dir_permissions.set_mode(0o777);
        fs::set_permissions(&device_photos_dir, dir_permissions).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to set directory permissions: {}", e)
            ))?;

        let file_path = format!("{}/{}", device_photos_dir, filename);
        println!("✅ Writing raw image to: {}", file_path);
        
        // Write file to disk (PRIMARY - for inference watcher)
        fs::write(&file_path, &image_bytes).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to save image: {}", e)
            ))?;

        // Set file permissions to 666 (rw-rw-rw-)
        let file_metadata = fs::metadata(&file_path).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to read file metadata: {}", e)
            ))?;
        let mut file_permissions = file_metadata.permissions();
        file_permissions.set_mode(0o666);
        fs::set_permissions(&file_path, file_permissions).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Failed to set file permissions: {}", e)
            ))?;

        // Create copy in 'latest' folder for dashboard (non-blocking, best effort)
        let latest_dir = format!("{}/latest", device_photos_dir);
        let latest_filename = format!("latest.{}", extension);
        let image_bytes_clone = image_bytes.clone();
        
        tokio::spawn(async move {
            if let Err(e) = Self::save_latest_copy(latest_dir, latest_filename, image_bytes_clone).await {
                eprintln!("⚠️  Failed to save latest copy (non-critical): {}", e);
            }
        });

        Ok(file_path)
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
