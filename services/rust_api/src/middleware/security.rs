use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{self, HeaderName, HeaderValue},
    Error,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};

// Middleware factory
pub struct SecurityHeadersMiddleware;

impl<S, B> Transform<S, ServiceRequest> for SecurityHeadersMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SecurityHeadersMiddlewareInner<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityHeadersMiddlewareInner {
            service: Rc::new(service),
        }))
    }
}

pub struct SecurityHeadersMiddlewareInner<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersMiddlewareInner<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        // Extract the origin header before moving the request
        let origin_header = req.headers().get(header::ORIGIN).cloned();

        Box::pin(async move {
            let mut res = svc.call(req).await?;

            let headers = res.headers_mut();

            // Add security headers
            headers.insert(
                header::STRICT_TRANSPORT_SECURITY,
                HeaderValue::from_static("max-age=31536000; includeSubDomains"),
            );
            headers.insert(
                HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_static("nosniff"),
            );
            headers.insert(
                HeaderName::from_static("x-frame-options"),
                HeaderValue::from_static("DENY"),
            );
            headers.insert(
                HeaderName::from_static("content-security-policy"),
                HeaderValue::from_static("default-src 'self'; img-src 'self' data:; script-src 'self'; object-src 'none'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'"),
            );
            headers.insert(
                HeaderName::from_static("x-xss-protection"),
                HeaderValue::from_static("1; mode=block"),
            );
            headers.insert(
                HeaderName::from_static("referrer-policy"),
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            );

            // Add CORS headers only for trusted origins
            let origin = origin_header;

            // List of trusted origins
            // Load trusted origins from environment or config
            let trusted_origins: Vec<String> = std::env::var("TRUSTED_ORIGINS")
                .unwrap_or_else(|_| String::from("https://eye-dar.com,https://app.eye-dar.com"))
                .split(',')
                .map(String::from)
                .collect();

            // Only add CORS headers if the origin is in our trusted list
            if let Some(origin) = origin {
                if let Ok(origin_str) = origin.to_str() {
                    if trusted_origins.contains(&origin_str.to_string()) {
                        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());
                        // Strict method restrictions
                        headers.insert(
                            header::ACCESS_CONTROL_ALLOW_METHODS,
                            HeaderValue::from_static("GET, POST"),
                        );
                        headers.insert(
                            header::ACCESS_CONTROL_MAX_AGE,
                            HeaderValue::from_static("3600"),
                        );
                        headers.insert(
                            header::ACCESS_CONTROL_ALLOW_HEADERS,
                            HeaderValue::from_static("Content-Type, Authorization"),
                        );
                        headers.insert(
                            header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                            HeaderValue::from_static("true"),
                        );
                        // Add other CORS headers as needed
                    }
                }
            }

            Ok(res)
        })
    }
}
