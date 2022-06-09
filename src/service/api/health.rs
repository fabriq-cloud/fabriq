use akira_core::{Health, HealthRequest, HealthResponse};
use tonic::{Request, Response, Status};

pub struct GrpcHealthService {}

impl GrpcHealthService {
    pub fn new() -> Self {
        GrpcHealthService {}
    }
}

#[tonic::async_trait]
impl Health for GrpcHealthService {
    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let response = HealthResponse { ok: true };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use akira_core::{Health, HealthRequest};
    use tonic::Request;

    use super::GrpcHealthService;

    #[tokio::test]
    async fn test_health_endpoint() -> anyhow::Result<()> {
        let health_service = GrpcHealthService::new();

        let request = Request::new(HealthRequest {});
        let response = health_service.health(request).await.unwrap().into_inner();

        assert!(response.ok);

        Ok(())
    }
}
