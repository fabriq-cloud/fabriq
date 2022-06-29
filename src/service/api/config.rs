use akira_core::{
    ConfigIdRequest, ConfigMessage, ConfigTrait, OperationId, QueryConfigRequest,
    QueryConfigResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Config;
use crate::services::ConfigService;

pub struct GrpcConfigService {
    service: Arc<ConfigService>,
}

impl GrpcConfigService {
    pub fn new(service: Arc<ConfigService>) -> Self {
        GrpcConfigService { service }
    }
}

#[tonic::async_trait]
impl ConfigTrait for GrpcConfigService {
    async fn create(
        &self,
        request: Request<ConfigMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let new_config: Config = request.into_inner().into();

        let operation_id = match self.service.create(&new_config, &None) {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::AlreadyExists,
                    format!("config {} already exists", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    async fn delete(
        &self,
        request: Request<ConfigIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: check that no workloads are currently still using config
        // Query workload service for workloads by config_id, error if any exist

        let operation_id = match self.service.delete(&request.into_inner().config_id, &None) {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("config with id {} not found", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    async fn query(
        &self,
        request: Request<QueryConfigRequest>,
    ) -> Result<Response<QueryConfigResponse>, Status> {
        let query = request.into_inner();
        let configs = match self.service.query(&query.deployment_id, &query.workload_id) {
            Ok(configs) => configs,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing configs failed with {}", err),
                ))
            }
        };

        let config_messages = configs.iter().map(|config| config.clone().into()).collect();

        let response = QueryConfigResponse {
            configs: config_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use akira_core::{
        ConfigIdRequest, ConfigMessage, ConfigTrait, EventStream, QueryConfigRequest,
    };
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcConfigService;

    use crate::persistence::memory::ConfigMemoryPersistence;
    use crate::services::ConfigService;

    #[tokio::test]
    async fn test_create_list_config() -> anyhow::Result<()> {
        let config_persistence = Box::new(ConfigMemoryPersistence::default());
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let config_service = Arc::new(ConfigService {
            persistence: config_persistence,
            event_stream,
        });

        let config_grpc_service = GrpcConfigService::new(Arc::clone(&config_service));

        let request = Request::new(ConfigMessage {
            id: "config-persist-single-under-test".to_owned(),

            owning_model: "workload:workload-fixture".to_owned(),

            key: "sample-key".to_owned(),
            value: "sample-value".to_owned(),
        });

        let response = config_grpc_service
            .create(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(QueryConfigRequest {
            deployment_id: "deployment-fixture".to_owned(),
            workload_id: "workload-fixture".to_owned(),
        });

        let _ = config_grpc_service
            .query(request)
            .await
            .unwrap()
            .into_inner();

        let request = Request::new(ConfigIdRequest {
            config_id: "config-persist-single-under-test".to_string(),
        });
        let response = config_grpc_service
            .delete(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        Ok(())
    }
}
