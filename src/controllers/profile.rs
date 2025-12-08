use std::sync::Arc;
use axum::Extension;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use crate::models::app_state::AppState;
use axum::http::StatusCode;

pub async fn update_favorite_team(State(app_state): State<Arc<AppState>>, Extension(user): Extension<crate::models::authentication_models::Claims>,Path(new_team):Path<String>) -> impl IntoResponse {
    tracing::info!("entered update_favorite_team controller");
    /*
        here we are going to keep tracks the changes of the favorite team of the user.
        updating those changes in the database.
    */

    tracing::info!("new team is {}", new_team);
    match app_state.database_connection.update_favorite_team(user.user_id, &new_team).await {
        Ok(_) => {
            tracing::info!("updated favorite team successfully");
            StatusCode::OK
        },
        Err(err) => {
            tracing::error!("error occurred while updating favorite team {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }

}