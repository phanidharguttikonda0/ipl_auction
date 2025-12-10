use std::sync::Arc;
use axum::extract::State;
use axum::{Extension, Form};
use axum::response::IntoResponse;
use crate::models::others::FeedBackRequest;
use axum::http::{StatusCode};
use crate::models::app_state::AppState;
use crate::models::authentication_models::Claims;
use axum::Json;
use serde_json::json;

pub async fn feed_back(
    State(app_state): State<Arc<AppState>>,
    Extension(user): Extension<Claims>,
    Json(feedback): Json<FeedBackRequest>,
) -> impl IntoResponse {
    tracing::info!("entered feedback controller");

    // Validate feedback_type
    let feedback_type = feedback.feedback_type.to_lowercase();
    if !["bug", "rating", "improvements"].contains(&feedback_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid feedback_type"})),
        );
    }

    // For rating: rating_value must exist
    if feedback_type == "rating" && feedback.rating_value.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "rating_value is required for rating feedback"})),
        );
    }

    // Validate title & description
    let title = match feedback.title.clone() {
        Some(t) => {
            if t.trim().is_empty() && feedback_type != "rating" {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "title is required"})),
                );
            }else if t.trim().is_empty() && feedback_type == "rating" {
                String::from("Nothing")
            }else {
                t
            }
        },
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "title is required"})),
            );
        }
    };

    let description = match feedback.description.clone() {
        Some(d) => {
            if d.trim().is_empty() && feedback_type != "rating" {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "description is required"})),
                );
            }else if d.trim().is_empty() && feedback_type == "rating" {
                String::from("Nothing")
            }else {
                d
            }
        },
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "description is required"})),
            );
        }
    };

    tracing::info!("validation completed successfully");

    // Get user_id from JWT
    let user_id = user.user_id.clone();

    // SQL insert
    let query = r#"
        INSERT INTO user_feedback (user_id, feedback_type, rating_value, title, description)
        VALUES ($1, $2::feedback_type_enum, $3, $4, $5)
    "#;

    let result = sqlx::query(query)
        .bind(user_id)
        .bind(feedback_type)
        .bind(feedback.rating_value)
        .bind(title)
        .bind(description)
        .execute(&app_state.database_connection.connection)
        .await;

    match result {
        Ok(_) => {
            tracing::info!("feedback insertion success");
            (
                StatusCode::OK,
                Json(json!({"message": "Feedback submitted successfully"})),
            )
        }
        Err(e) => {
            tracing::error!("DB Error inserting feedback: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to submit feedback"})),
            )
        }
    }
}
