use axum::response::Response;
use axum::middleware::Next;

use axum::{
    http::Request,
    body::Body,
};

pub async fn auth_check(
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // your logic
    next.run(req).await
}
