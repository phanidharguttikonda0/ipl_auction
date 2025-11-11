use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde_json::json;
use crate::models::authentication_models::Claims;

pub async fn auth_check(mut req: Request, next: Next) -> Response {
    // Extract Authorization header
    let Some(header_value) = req.headers().get("Authorization") else {
        return unauthorized("Authorization header is missing");
    };

    // Convert to str
    let Ok(header_str) = header_value.to_str() else {
        return unauthorized("Failed to convert Authorization header to str");
    };

    // Expect "Bearer <token>"
    let Some(token) = header_str.strip_prefix("Bearer ") else {
        return unauthorized("Only Bearer token is supported");
    };

    // Decode and verify JWT
    let secret_key = std::env::var("SECRET_KEY").unwrap();
    let key = DecodingKey::from_secret(secret_key.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    let decoded = match decode::<Claims>(token, &key, &validation) {
        Ok(data) => data.claims,
        Err(_) => {
            tracing::error!("Token expired or invalid secret key");
            return unauthorized("Token has expired or is invalid");
        },
    };

    // Attach claims to request extensions
    req.extensions_mut().insert(decoded);

    // Continue with the request
    next.run(req).await
}

// Helper function for consistent unauthorized responses
fn unauthorized(msg: &str) -> Response {
    let body = json!({ "message": msg });
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body).unwrap().into())
        .unwrap()
}
