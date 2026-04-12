use crate::services::{
    review_queue::{
        get_pending_reviews_handler, get_review_handler, queue_detection_handler,
        update_review_status_handler,
    },
    yolo_training::{get_job_status_handler, test_handler, trigger_training_handler},
};
use actix_web::web;

pub fn create_yolo_scope(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        web::scope("/yolo")
            .service(test_handler)
            .service(trigger_training_handler)
            .service(
                web::scope("/review")
                    .route("", web::get().to(get_pending_reviews_handler))
                    .route("/{image_path:.*}", web::post().to(queue_detection_handler))
                    .route("/{id}", web::get().to(get_review_handler))
                    .route("/{id}/status", web::put().to(update_review_status_handler)),
            )
            .service(get_job_status_handler),
    );
}
