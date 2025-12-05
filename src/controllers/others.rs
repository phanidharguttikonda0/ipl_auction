use axum::Form;
use axum::response::IntoResponse;
use crate::models::others::FeedBackRequest;
use axum::http::{StatusCode};
pub async fn feed_back(Form(feedback): Form<FeedBackRequest>) -> impl IntoResponse {
    StatusCode::OK
}