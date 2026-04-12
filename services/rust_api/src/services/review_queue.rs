use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use super::minio_client::MinioClient;
use crate::models::review_queue::{ReviewItem, ReviewStatus, UnclassifiedDetection};

const CONFIDENCE_THRESHOLD: f32 = 0.6;
const REVIEW_BUCKET: &str = "review-queue";

#[derive(Clone)]
pub struct ReviewQueueService {
    items: Arc<Mutex<Vec<ReviewItem>>>,
    storage: MinioClient,
}

#[derive(Debug, Serialize)]
pub struct ReviewQueueResponse {
    message: String,
    item_id: Option<String>,
}

impl ReviewQueueService {
    pub async fn new(storage: MinioClient) -> Result<Self, Box<dyn std::error::Error>> {
        let service = Self {
            items: Arc::new(Mutex::new(Vec::new())),
            storage,
        };

        // Load existing items from MinIO
        let items = service.load_items_from_storage().await?;
        *service.items.lock().unwrap() = items;

        Ok(service)
    }

    async fn load_items_from_storage(&self) -> Result<Vec<ReviewItem>, Box<dyn std::error::Error>> {
        let mut items = Vec::new();

        // List all review items in the bucket
        let list_result = self
            .storage
            .client
            .list_objects_v2()
            .bucket(REVIEW_BUCKET)
            .send()
            .await?;

        if let Some(contents) = list_result.contents {
            for item in contents {
                if let Some(key) = item.key {
                    if key.ends_with(".json") {
                        if let Ok(data) = self.storage.get_file(&key).await {
                            if let Ok(item) = serde_json::from_slice::<ReviewItem>(&data) {
                                items.push(item);
                            }
                        }
                    }
                }
            }
        }

        Ok(items)
    }

    pub async fn queue_detection(
        &self,
        image_path: String,
        predictions: Vec<UnclassifiedDetection>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let item = ReviewItem::new(image_path.clone(), predictions);
        let item_id = item.id.clone();

        // Store the review item in MinIO
        let json_data = serde_json::to_string(&item)?;
        self.storage
            .put_object(
                REVIEW_BUCKET,
                &format!("{}/{}.json", Utc::now().format("%Y%m%d"), item_id),
                json_data.as_bytes().to_vec(),
                Some("application/json".to_string()),
            )
            .await?;

        // Add to in-memory queue
        self.items.lock().unwrap().push(item);

        Ok(item_id)
    }

    pub fn get_pending_reviews(&self) -> Vec<ReviewItem> {
        self.items
            .lock()
            .unwrap()
            .iter()
            .filter(|item| item.status == ReviewStatus::Pending)
            .cloned()
            .collect()
    }

    pub fn get_review_by_id(&self, id: &str) -> Option<ReviewItem> {
        self.items
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.id == id)
            .cloned()
    }

    pub async fn update_review_status(
        &self,
        id: &str,
        status: ReviewStatus,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // First find and update the item in memory, collecting necessary data
        let storage_update = {
            let mut items = self.items.lock().unwrap();
            if let Some(item) = items.iter_mut().find(|i| i.id == id) {
                item.status = status.clone();

                // Prepare storage update data
                let json_data = serde_json::to_string(&item)?;
                let file_path = format!("{}/{}.json", item.created_at.format("%Y%m%d"), item.id);
                Some((json_data, file_path))
            } else {
                None
            }
        }; // MutexGuard is dropped here

        // If we found an item, update storage
        if let Some((json_data, file_path)) = storage_update {
            self.storage
                .put_object(
                    REVIEW_BUCKET,
                    &file_path,
                    json_data.as_bytes().to_vec(),
                    Some("application/json".to_string()),
                )
                .await?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn should_queue_for_review(predictions: &[UnclassifiedDetection]) -> bool {
        predictions
            .iter()
            .any(|p| p.confidence < CONFIDENCE_THRESHOLD)
    }
}

// API handlers

pub async fn queue_detection_handler(
    service: web::Data<ReviewQueueService>,
    payload: web::Json<Vec<UnclassifiedDetection>>,
    image_path: web::Path<String>,
) -> impl Responder {
    let image_path = image_path.into_inner();
    match service
        .queue_detection(image_path, payload.into_inner())
        .await
    {
        Ok(item_id) => HttpResponse::Ok().json(ReviewQueueResponse {
            message: "Detection queued for review".to_string(),
            item_id: Some(item_id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ReviewQueueResponse {
            message: format!("Failed to queue detection: {e}"),
            item_id: None,
        }),
    }
}

pub async fn get_pending_reviews_handler(service: web::Data<ReviewQueueService>) -> impl Responder {
    HttpResponse::Ok().json(service.get_pending_reviews())
}

pub async fn get_review_handler(
    service: web::Data<ReviewQueueService>,
    id: web::Path<String>,
) -> impl Responder {
    match service.get_review_by_id(&id) {
        Some(item) => HttpResponse::Ok().json(item),
        None => HttpResponse::NotFound().json(ReviewQueueResponse {
            message: "Review item not found".to_string(),
            item_id: None,
        }),
    }
}

#[derive(Deserialize)]
pub struct StatusUpdate {
    status: ReviewStatus,
}

pub async fn update_review_status_handler(
    service: web::Data<ReviewQueueService>,
    id: web::Path<String>,
    payload: web::Json<StatusUpdate>,
) -> impl Responder {
    match service
        .update_review_status(&id, payload.status.clone())
        .await
    {
        Ok(true) => HttpResponse::Ok().json(ReviewQueueResponse {
            message: "Review status updated".to_string(),
            item_id: Some(id.to_string()),
        }),
        Ok(false) => HttpResponse::NotFound().json(ReviewQueueResponse {
            message: "Review item not found".to_string(),
            item_id: None,
        }),
        Err(e) => HttpResponse::InternalServerError().json(ReviewQueueResponse {
            message: format!("Failed to update review status: {e}"),
            item_id: None,
        }),
    }
}
