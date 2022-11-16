use axum::{routing::get, Router};

pub mod health;
pub mod webhook;

pub fn http_router() -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/event_handler", get(webhook::event_handler))
}
