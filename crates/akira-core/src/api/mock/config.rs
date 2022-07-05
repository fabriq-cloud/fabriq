use tonic::{Request, Response, Status};

use crate::{
    ConfigIdRequest, ConfigMessage, ConfigTrait, OperationId, QueryConfigRequest,
    QueryConfigResponse,
};

pub struct MockConfigClient {}

#[tonic::async_trait]
impl ConfigTrait for MockConfigClient {
    async fn create(
        &self,
        _request: Request<ConfigMessage>,
    ) -> Result<Response<OperationId>, Status> {
        Ok(Response::new(OperationId::create()))
    }

    async fn delete(
        &self,
        _request: Request<ConfigIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        Ok(Response::new(OperationId::create()))
    }

    async fn query(
        &self,
        _request: Request<QueryConfigRequest>,
    ) -> Result<Response<QueryConfigResponse>, Status> {
        let configs = vec![
            ConfigMessage {
                id: "deployment-fixture:replicas".to_owned(),
                owning_model: "deployment:deployment-fixture".to_owned(),
                key: "replicas".to_owned(),
                value: "5".to_owned(),
            },
            ConfigMessage {
                id: "workload-fixture:port".to_owned(),
                owning_model: "workload:workload-fixture".to_owned(),
                key: "port".to_owned(),
                value: "80".to_owned(),
            },
            ConfigMessage {
                id: "deployment-fixture:image".to_owned(),
                owning_model: "deployment:deployment-fixture".to_owned(),
                key: "image".to_owned(),
                value: "ghcr.io/timfpark/akira-gitops:aa14d4371cfc107bb5cc35d2cade57896841e0f9"
                    .to_owned(),
            },
            ConfigMessage {
                id: "workload-fixture:metricsEndpoint".to_owned(),
                owning_model: "workload:workload-fixture".to_owned(),
                key: "metricsEndpoint".to_owned(),
                value: "/metrics".to_owned(),
            },
            ConfigMessage {
                id: "workload-fixture:healthEndpoint".to_owned(),
                owning_model: "workload:workload-fixture".to_owned(),
                key: "healthEndpoint".to_owned(),
                value: "/healthz".to_owned(),
            },
        ];

        Ok(Response::new(QueryConfigResponse { configs }))
    }
}
