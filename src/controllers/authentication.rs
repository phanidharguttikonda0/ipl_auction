use std::sync::Arc;
use axum::extract::State;
use axum::{Form, Json};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use sqlx::Row;
use crate::models::app_state::AppState;
use crate::models::authentication_models::{AuthenticationModel};
use crate::services::other::create_authorization_header;

pub async fn authentication_handler(State(app_state): State<Arc<AppState>>, Form(details): Form<AuthenticationModel>) -> Result<Response, StatusCode> {
    // when we get the data, it will check whether the data exists or not, if exists then favorite team column
    // is not null. Then it will send the authorization header, else it will not send it so the front-end needs to
    // show the choice favorite team page and continue, once he/she sent the favorite team, then this controller
    // will send the authorization header in the response body

    // as it was staging and testing, we can write a dummy code here; that works for simple purpose

    // first thing to check whether that google sid exists , if not add it and return authorization header
    let gmail = details.gmail.trim();
    let username = gmail.split('@').next().unwrap_or("").to_string();
    tracing::info!("got the username as {}", username) ;
    let maybe_row = sqlx::query(
        r#"
        INSERT INTO users (username, mail_id, google_sid, favorite_team)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (google_sid)
        DO NOTHING
        RETURNING id, favorite_team;
        "#
    )
        .bind(&username)
        .bind(&gmail)
        .bind(&details.google_sid)
        .bind(details.favorite_team.unwrap_or_else(|| "".to_string()))
        .fetch_optional(&app_state.database_connection.connection)
        .await.unwrap();

    let id ;
    let favorite_team ;
    // 2️⃣ If inserted successfully, return new id & team
    if let Some(row) = maybe_row {
        tracing::info!("new user , so inserted and got the inserted values") ;
        id = row.get("id") ;
        favorite_team = row.get("favorite_team") ;
    }else {
        // 3️⃣ If conflict occurred (already exists), fetch existing record
        tracing::info!("old user, so getting the values") ;
        let row = sqlx::query(
            r#"
        SELECT id, favorite_team
        FROM users
        WHERE google_sid = $1;
        "#
        )
            .bind(&details.google_sid)
            .fetch_one(&app_state.database_connection.connection)
            .await.unwrap();
        id = row.get("id") ;
        favorite_team = row.get("favorite_team") ;
    }

    // we are going to get the authorization header
    match create_authorization_header(id, username, gmail.clone().parse().unwrap(), favorite_team) {
        Ok(auth_header) =>{
            tracing::info!("received auth header") ;
            // build headers
            let mut headers = HeaderMap::new();
            let bearer_value = format!("Bearer {}", auth_header);
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&bearer_value).unwrap(),
            );

            // create response body
            let body = Json(json!({
                "message": "Login successful",
            }));

            Ok((headers, body).into_response())
        },
        Err(err) => {
            tracing::error!("Failed to create the authorization header") ;
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": "Authorization Failed"
            }))).into_response())

        }
    }

}