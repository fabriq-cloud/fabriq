use axum::response::Html;

#[tracing::instrument(name = "http::event_handler", skip_all)]
pub async fn event_handler() -> Html<&'static str> {
    Html("ok")
}
