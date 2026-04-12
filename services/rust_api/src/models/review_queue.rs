use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewItem {
    pub id: String,
    pub image_path: String,
    pub predictions: Vec<UnclassifiedDetection>,
    pub created_at: DateTime<Utc>,
    pub status: ReviewStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnclassifiedDetection {
    pub bbox: [f32; 4], // [x, y, width, height]
    pub confidence: f32,
    pub class_id: i32,
    pub class_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReviewStatus {
    Pending,
    InProgress,
    Completed,
    Rejected,
}

impl ReviewItem {
    pub fn new(image_path: String, predictions: Vec<UnclassifiedDetection>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            image_path,
            predictions,
            created_at: Utc::now(),
            status: ReviewStatus::Pending,
        }
    }
}
