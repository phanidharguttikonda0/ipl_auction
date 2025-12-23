use axum::body::Body;
use axum::http::Request;
use tower_http::trace::{HttpMakeClassifier, TraceLayer};
use tracing::{info_span, Span};
use uuid::Uuid;

pub fn http_trace_layer() -> TraceLayer<HttpMakeClassifier, fn(&Request<Body>) -> Span> {
    TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
            let trace_id = Uuid::new_v4();

            info_span!(
                "http_request",
                trace_id = %trace_id,
                method = %request.method(),
                path = %request.uri().path(),
                user_agent = ?request.headers().get("user-agent"),
            )
        })
}
