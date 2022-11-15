use axum::response::Html;

#[tracing::instrument(name = "http::health")]
pub async fn health() -> Html<&'static str> {
    Html("ok")
}
