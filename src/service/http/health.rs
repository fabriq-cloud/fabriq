use axum::response::Html;

#[tracing::instrument(name = "http::health")]
pub async fn health() -> Html<&'static str> {
    Html("ok")
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{self, StatusCode},
    };
    use tower::ServiceExt;

    use crate::http::http_router;

    #[tokio::test]
    async fn test_health() {
        let app = http_router();

        let response = app
            .oneshot(
                http::Request::builder()
                    .method(http::Method::GET)
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(&body[..], b"ok");
    }
}
