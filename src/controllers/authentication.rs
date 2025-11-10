use std::sync::Arc;
use axum::extract::State;
use axum::Form;
use axum::response::Response;
use crate::models::app_state::AppState;
use crate::models::authentication_models::{AuthenticationModel};

pub async fn authentication_handler(State(app_state): State<Arc<AppState>>, Form(details): Form<AuthenticationModel>) -> Response {
    // when we get the data, it will check whether the data exists or not , if exists then favorite team column
    // is not null and then it will send authorization header else it will not send so the front-end needs to 
    // show the choose favorite team page and continue, once he/she sent the favorite team then this controller
    // will send the authorization header in the response body
}
