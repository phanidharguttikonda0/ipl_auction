use std::net::SocketAddr;
use std::sync::Arc;
use axum::extract::{ConnectInfo, State};
use axum::{Form, Json};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use sqlx::Row;
use crate::models::app_state::AppState;
use crate::models::authentication_models::{AuthenticationModel};
use crate::models::background_db_tasks::{DBCommandsAuction, UserExternalDetails};
use crate::services::other::create_authorization_header;
/*
     ip address was ::ffff:192.168.30.11 is an example i got from the logs of the server
*/
pub async fn authentication_handler(
    State(app_state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Form(details): Form<AuthenticationModel>,
) -> Result<Response, StatusCode> {
    let ip_address = addr.ip() ;
    tracing::info!("*****************************************") ;
    tracing::info!("ip address was {}", ip_address) ;
    tracing::info!("******************************************") ;
    let gmail = details.gmail.trim();
    let username = gmail.split('@').next().unwrap_or("").to_string();

    tracing::info!("got username {}", username);

    // ────────────────────────────────────────────────────────────────
    // 1️⃣ CHECK IF USER ALREADY EXISTS
    // ────────────────────────────────────────────────────────────────
    let existing_user = sqlx::query(
        r#"
        SELECT id, favorite_team, mail_id
        FROM users
        WHERE google_sid = $1
        "#,
    )
        .bind(&details.google_sid)
        .fetch_optional(&app_state.database_connection.connection)
        .await
        .unwrap();

    let (id, favorite_team);

    if let Some(row) = existing_user {
        // ─────────────────────────────────────────────────────────────
        // 2️⃣ OLD USER → No need for favorite_team
        // ─────────────────────────────────────────────────────────────
        tracing::info!("Old user detected");

        id = row.get("id");
        favorite_team = row.get::<String, _>("favorite_team");

    } else {
        // ─────────────────────────────────────────────────────────────
        // 3️⃣ NEW USER → favorite_team is REQUIRED
        // ─────────────────────────────────────────────────────────────
        /*

            over here we are going to add a message to the message queue to add the players geolocations


        */
        if details.favorite_team.is_none() || details.favorite_team.as_ref().unwrap().is_empty() {
            tracing::warn!("New user did NOT send favorite_team → REJECTING");
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "favorite_team is required for first-time signup"
                })),
            )
                .into_response());
        }

        // Only now do we insert (safe)
        let row = sqlx::query(
            r#"
            INSERT INTO users (username, mail_id, google_sid, favorite_team)
            VALUES ($1, $2, $3, $4)
            RETURNING id, favorite_team;
            "#,
        )
            .bind(&username)
            .bind(&gmail)
            .bind(&details.google_sid)
            .bind(details.favorite_team.as_ref().unwrap())
            .fetch_one(&app_state.database_connection.connection)
            .await
            .unwrap();

        tracing::info!("Inserted NEW USER successfully");

        tracing::info!("Now we are going to add this favorite team change to the database_task_executor message passing Queue") ;


        id = row.get("id");
        favorite_team = row.get("favorite_team");

        app_state.database_task_executor.send(DBCommandsAuction::AddUserExternalDetails(
            UserExternalDetails {
                user_id: id,
                ip_address: ip_address.to_string(),
            }
        )).expect("Unable to send the message to the db task executor");
    }

    // ────────────────────────────────────────────────────────────────
    // 4️⃣ Create Authorization Header (unchanged)
    // ────────────────────────────────────────────────────────────────
    match create_authorization_header(id, username, gmail.parse().unwrap(), favorite_team) {
        Ok(auth_header) => {
            tracing::info!("created auth header");

            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", auth_header)).unwrap(),
            );

            Ok((
                headers,
                Json(json!({
                    "message": "Login successful"
                })),
            )
                .into_response())
        }
        Err(err) => {
            tracing::error!("Failed to create the authorization header: {:?}", err);

            Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Authorization Failed"
                })),
            )
                .into_response())
        }
    }
}
