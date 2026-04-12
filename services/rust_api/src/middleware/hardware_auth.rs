use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::ErrorUnauthorized;
use actix_web::http::header;
use actix_web::Error;
use actix_web::HttpMessage;
use futures_util::future::{ok, Ready};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // hardware_id
    exp: usize,  // expiration time
}

pub struct HardwareAuth;

impl HardwareAuth {
    pub fn new() -> Self {
        HardwareAuth
    }
}

impl Default for HardwareAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B> Transform<S, ServiceRequest> for HardwareAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = HardwareAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(HardwareAuthMiddleware { service })
    }
}

pub struct HardwareAuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for HardwareAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Skip authentication for health check endpoint
        if req.path() == "/health" {
            return Box::pin(self.service.call(req));
        }

        // Get JWT secret from environment
        let jwt_secret = match env::var("JWT_SECRET") {
            Ok(s) => s,
            Err(_) => {
                tracing::error!("JWT_SECRET not configured!");
                return Box::pin(async move {
                    Err(ErrorUnauthorized("Server authentication not configured"))
                });
            }
        };

        // Get client IP for logging
        let client_ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();

        // Get token from Authorization header
        let auth_header = match req.headers().get(header::AUTHORIZATION) {
            Some(h) => h,
            None => {
                tracing::warn!(
                    target: "auth_failures",
                    "No Authorization header found - IP: {}, Path: {}, Headers: {:?}",
                    client_ip,
                    req.path(),
                    req.headers()
                );
                return Box::pin(async move {
                    Err(ErrorUnauthorized("No authorization token provided"))
                });
            }
        };

        // Parse Bearer token
        let auth_str = match auth_header.to_str() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    target: "auth_failures",
                    "Invalid authorization header - IP: {}, Path: {}, Error: {}",
                    client_ip,
                    req.path(),
                    e
                );
                return Box::pin(
                    async move { Err(ErrorUnauthorized("Invalid authorization header")) },
                );
            }
        };

        if !auth_str.starts_with("Bearer ") {
            return Box::pin(async move { Err(ErrorUnauthorized("Invalid authorization scheme")) });
        }

        let token = &auth_str[7..];

        // Validate JWT token
        let validation = Validation::new(Algorithm::HS256);
        let token_data = match decode::<Claims>(
            token,
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &validation,
        ) {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Token validation failed: {}", e);
                return Box::pin(async move { Err(ErrorUnauthorized("Invalid token")) });
            }
        };

        // Log successful authentication
        tracing::info!(
            "Authenticated hardware ID: {} for path: {}",
            token_data.claims.sub,
            req.path()
        );

        // Add hardware ID to request extensions for use in handlers
        req.extensions_mut().insert(token_data.claims.sub);

        Box::pin(self.service.call(req))
    }
}
