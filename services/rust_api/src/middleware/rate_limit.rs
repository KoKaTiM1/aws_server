use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::InternalError,
    http::StatusCode,
    Error,
};
use futures_util::future::LocalBoxFuture;
use std::{
    collections::HashMap,
    future::{ready, Ready},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

// Rate limit configuration
#[derive(Clone)]
pub struct RateLimiter {
    requests_per_minute: u32,
    clients: Arc<Mutex<HashMap<String, RateLimit>>>,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        RateLimiter {
            requests_per_minute,
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[derive(Debug)]
struct RateLimit {
    window_start: Instant, // Start time of current window
    request_count: u32,    // Current count of requests in window
    max_requests: u32,     // Maximum requests allowed per window
}

impl RateLimit {
    fn new(max_requests: u32) -> Self {
        RateLimit {
            window_start: Instant::now(),
            request_count: 0, // Start at 0, will increment on first check
            max_requests,
        }
    }

    fn check_and_increment(&mut self, now: Instant) -> bool {
        // First check if we need to reset the window
        if now.duration_since(self.window_start) >= Duration::from_secs(60) {
            self.window_start = now;
            self.request_count = 1; // Count this first request in new window
            false // Allow this request
        } else if self.request_count >= self.max_requests {
            true // Block this request, we're at/over limit
        } else {
            self.request_count += 1; // Increment first, then allow
            false // Allow this request
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddleware {
            service: Arc::new(service),
            requests_per_minute: self.requests_per_minute,
            clients: self.clients.clone(),
        }))
    }
}

pub struct RateLimitMiddleware<S> {
    service: Arc<S>,
    requests_per_minute: u32,
    clients: Arc<Mutex<HashMap<String, RateLimit>>>,
}

impl<S> Clone for RateLimitMiddleware<S> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            requests_per_minute: self.requests_per_minute,
            clients: self.clients.clone(),
        }
    }
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();

        let clients = self.clients.clone();
        let requests_per_minute = self.requests_per_minute;
        let service = self.service.clone();

        Box::pin(async move {
            let now = Instant::now();
            let mut clients = clients.lock().await;

            // Get or create rate limit entry for this IP
            let rate_limit = clients
                .entry(ip.clone())
                .or_insert_with(|| RateLimit::new(requests_per_minute));

            // First check if we're over the limit
            let is_limited = rate_limit.check_and_increment(now);

            if is_limited {
                return Err(Error::from(InternalError::new(
                    format!(
                        "Rate limit exceeded - maximum {requests_per_minute} requests per minute"
                    ),
                    StatusCode::TOO_MANY_REQUESTS,
                )));
            }

            drop(clients); // Drop the mutex before async operation

            // Process request if rate limit not exceeded
            service.call(req).await
        })
    }
}
