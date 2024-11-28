use axum::{routing, Router};

pub fn make_router() -> Router {
    Router::new().route("/ping", routing::get(|| async { "pong" }))
}
