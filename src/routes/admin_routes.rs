use axum::Router;
use axum::routing::get;
use crate::controllers::admin::get_player;

pub async fn admin_routes() -> Router{
    Router::new()
        .route("/get-redis-player/{player_id}", get(get_player))
}