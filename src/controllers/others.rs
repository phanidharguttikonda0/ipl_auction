use std::sync::Arc;
use axum::extract::State;
use axum::{Extension, Form};
use axum::response::IntoResponse;
use crate::models::others::FeedBackRequest;
use axum::http::{StatusCode};
use crate::models::app_state::AppState;
use crate::models::authentication_models::Claims;

pub async fn feed_back(
    State(app_state): State<Arc<AppState>>,
    Extension(user): Extension<Claims>,
    Form(feedback): Form<FeedBackRequest>,
) -> impl IntoResponse {
    use axum::Json;
    use serde_json::json;

    tracing::info!("validating feed back type") ;
    // 1. Validate feedback_type
    let feedback_type = feedback.feedback_type.to_lowercase();
    if !["bug", "rating", "improvements"].contains(&feedback_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid feedback_type"})),
        );
    }

    tracing::info!("checking rating value exists or not") ;
    // 2. For rating: rating_value must exist
    if feedback_type == "rating" && feedback.rating_value.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "rating_value is required for rating feedback"})),
        );
    }

    tracing::info!("validating title and description") ;
    // 3. Validate title and description
    let title = match feedback.title {
        Some(t) => t,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "title is required"})),
            )
        }
    };

    let description = match feedback.description {
        Some(d) => d,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "description is required"})),
            )
        }
    };

    tracing::info!("extracting user_id from jwt") ;
    // 4. Extract user_id from JWT claims
    let user_id = user.user_id.clone();

    // 5. Insert into database using sqlx
    let query = r#"
        INSERT INTO user_feedback (user_id, feedback_type, rating_value, title, description)
        VALUES ($1, $2, $3, $4, $5)
    "#;

    let result = sqlx::query(query)
        .bind(user_id)
        .bind(feedback_type)
        .bind(feedback.rating_value)
        .bind(title)
        .bind(description)
        .execute(&app_state.database_connection.connection)
        .await;
    tracing::info!("query executed") ;
    match result {
        Ok(_) => {
            tracing::info!("query success") ;
            (
                StatusCode::OK,
                Json(json!({"message": "Feedback submitted successfully"})),
            )
        },
        Err(e) => {
            tracing::info!("Error inserting feedback: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to submit feedback"})),
            )
        }
    }
}
