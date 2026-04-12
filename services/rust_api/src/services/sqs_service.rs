use aws_sdk_sqs::Client as SqsClient;
use serde_json::json;

pub struct SqsService;

impl SqsService {
    /// Publish a detection_created event to SQS
    pub async fn publish_detection_created(
        sqs_client: &SqsClient,
        queue_url: &str,
        device_id: u32,
        detection_images: Vec<String>, // S3 URIs
        message: &str,
        timestamp: &str,
        severity: &str,
        sensor_source: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let payload = json!({
            "event": "detection_created",
            "device_id": device_id,
            "detection_images": detection_images,
            "message": message,
            "timestamp": timestamp,
            "severity": severity,
            "sensor_source": sensor_source.unwrap_or("unknown"),
        });

        let message_body = serde_json::to_string(&payload)?;

        let response = sqs_client
            .send_message()
            .queue_url(queue_url)
            .message_body(message_body)
            .send()
            .await?;

        let message_id = response.message_id().unwrap_or_default().to_string();
        println!("✅ Published detection_created to SQS - MessageId: {}", message_id);

        Ok(message_id)
    }

    /// Publish a verify_requested event to SQS
    pub async fn publish_verify_requested(
        sqs_client: &SqsClient,
        queue_url: &str,
        detection_id: String,
        device_id: u32,
        images: Vec<String>, // S3 URIs
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let payload = json!({
            "event": "verify_requested",
            "detection_id": detection_id,
            "device_id": device_id,
            "images": images,
        });

        let message_body = serde_json::to_string(&payload)?;

        let response = sqs_client
            .send_message()
            .queue_url(queue_url)
            .message_body(message_body)
            .send()
            .await?;

        let message_id = response.message_id().unwrap_or_default().to_string();
        println!("✅ Published verify_requested to SQS - MessageId: {}", message_id);

        Ok(message_id)
    }

    /// Publish a verified_animals event to SQS (for notifications)
    pub async fn publish_verified_animals(
        sqs_client: &SqsClient,
        queue_url: &str,
        detection_id: String,
        device_id: u32,
        verified_animals: Vec<String>,
        confidence: f32,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let payload = json!({
            "event": "verified_animals",
            "detection_id": detection_id,
            "device_id": device_id,
            "verified_animals": verified_animals,
            "confidence": confidence,
        });

        let message_body = serde_json::to_string(&payload)?;

        let response = sqs_client
            .send_message()
            .queue_url(queue_url)
            .message_body(message_body)
            .send()
            .await?;

        let message_id = response.message_id().unwrap_or_default().to_string();
        println!("✅ Published verified_animals to SQS - MessageId: {}", message_id);

        Ok(message_id)
    }
}
