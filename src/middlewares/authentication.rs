use axum::response::Response;
use axum::middleware::Next;

pub async fn auth_check<B>(mut req: axum::http::Request<B>, next: Next) -> Response {
    let stage = std::env::var("STAGE").expect("STAGE is not set") ;

    next.run(req).await
}