//! Middleware functions for the Relay Server API
//!
//! Provides middleware for adding Private Network Access (PNA) headers
//! to enable HTTPS pages to access local network resources.

use axum::{body::Body, http::{header::HeaderValue, Request}, middleware, response::Response};

/// Middleware to add PNA (Private Network Access) headers
///
/// Adds Access-Control-Allow-Private-Network header to responses
/// for browsers to allow HTTPS pages to access local network resources.
pub async fn add_pna_headers(request: Request<Body>, next: middleware::Next) -> Response {
    let mut response = next.run(request).await;

    // Add PNA header to allow private network access
    // This is required for Chrome's Private Network Access feature
    response.headers_mut().insert(
        "Access-Control-Allow-Private-Network",
        HeaderValue::from_static("true"),
    );

    response
}
