use axum::response::Response;
use axum::middleware::Next;

use axum::{http::Request, body::Body, Json};
use axum::http::StatusCode;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde_json::json;
use crate::models::authentication_models::Claims;

pub async fn auth_check(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, serde_json::value::Value)> {
    // Extract Authorization header
    let Some(header_value) = req.headers().get("Authorization") else {
        return Err((StatusCode::UNAUTHORIZED, json!({
            "message": "Authorization header is missing",
        })));
    };

    // Convert to str
    let Ok(header_str) = header_value.to_str() else {
        tracing::error!("Failed to convert Authorization header to str");
        return Err((StatusCode::UNAUTHORIZED, json!({
            "message": "Failed to convert Authorization header to str",
        })));
    };
    let secret_key = std::env::var("SECRET_KEY").unwrap();
    // Expect "Bearer <token>"
    let Some(token) = header_str.strip_prefix("Bearer ") else {
        return Err((StatusCode::UNAUTHORIZED, json!({
            "message": "Only Bearer token is supported",
        })));
    };

    // Decode and verify JWT
    let key = DecodingKey::from_secret(secret_key.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    let decoded = match decode::<Claims>(token, &key, &validation) {
        Ok(data) => data.claims,
        Err(_) => {
            tracing::error!("Token has Expired or the Secret key provided was changes") ;
            return Err((StatusCode::UNAUTHORIZED, json!({
            "message": "Token has Expired",
        })))
        },
    };

    // attached the decoded claims to the request 
    req.extensions_mut().insert(decoded);
    Ok(next.run(req).await)
}
