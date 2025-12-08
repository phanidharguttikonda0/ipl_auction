use std::sync::Arc;
use axum::{Extension, Json};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use crate::models::app_state::AppState;
use axum::http::StatusCode;
use http::{HeaderMap, HeaderValue};
use serde_json::json;
use crate::services::other::create_authorization_header;

pub async fn update_favorite_team(State(app_state): State<Arc<AppState>>, Extension(mut user): Extension<crate::models::authentication_models::Claims>, Path(new_team):Path<String>) -> impl IntoResponse {
    tracing::info!("entered update_favorite_team controller");
    /*
        here we are going to keep tracks the changes of the favorite team of the user.
        updating those changes in the database.
    */

    tracing::info!("new team is {}", new_team);
    match app_state.database_connection.update_favorite_team(user.user_id, &new_team).await {
        Ok(_) => {
            tracing::info!("updated favorite team successfully");
            user.favorite_team = new_team ;
            let auth_header = create_authorization_header(user.user_id, user.username,user.gmail, user.favorite_team).expect("unable to create authorization header") ;
            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", auth_header)).unwrap(),
            );

            (
                StatusCode::OK,
                headers
            ).into_response()
        },
        Err(err) => {
            tracing::error!("error occurred while updating favorite team {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
    /*
         regenerating the authorization header and passing it as response
    */
}