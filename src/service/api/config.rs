use akira_core::{
    ConfigIdRequest, ConfigMessage, ConfigTrait, OperationId, QueryConfigRequest,
    QueryConfigResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Config;
use crate::services::ConfigService;

#[derive(Debug)]
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
    #[tracing::instrument(name = "grpc::config::upsert")]
    async fn upsert(
        &self,
        request: Request<ConfigMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let new_config: Config = request.into_inner().into();

        let operation_id = match self.service.upsert(&new_config, &None).await {
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

    #[tracing::instrument(name = "grpc::config::delete")]
    async fn delete(
        &self,
        request: Request<ConfigIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: check that no workloads are currently still using config
        // Query workload service for workloads by config_id, error if any exist

        let operation_id = match self
            .service
            .delete(&request.into_inner().config_id, &None)
            .await
        {
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

    #[tracing::instrument(name = "grpc::config::query")]
    async fn query(
        &self,
        request: Request<QueryConfigRequest>,
    ) -> Result<Response<QueryConfigResponse>, Status> {
        let query = request.into_inner();

        let configs = match self.service.query(&query).await {
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
    use akira_core::test::{
        get_deployment_fixture, get_string_config_fixture, get_target_fixture,
        get_template_fixture, get_workload_fixture,
    };
    use akira_core::{ConfigIdRequest, ConfigTrait, EventStream, QueryConfigRequest};
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcConfigService;

    use crate::models::{Deployment, Target, Template, Workload};
    use crate::persistence::memory::{
        ConfigMemoryPersistence, DeploymentMemoryPersistence, MemoryPersistence,
        WorkloadMemoryPersistence,
    };
    use crate::services::{
        ConfigService, DeploymentService, TargetService, TemplateService, WorkloadService,
    };

    #[tokio::test]
    async fn test_create_list_config() -> anyhow::Result<()> {
        let config_persistence = Box::new(ConfigMemoryPersistence::default());
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let template_persistence = MemoryPersistence::<Template>::default();
        let template_service = Arc::new(TemplateService {
            persistence: Box::new(template_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let template: Template = get_template_fixture(Some("template-fixture")).into();
        let operation_id = template_service.upsert(&template, None).await.unwrap();

        let workload_persistence = Box::new(WorkloadMemoryPersistence::default());
        let workload_service = Arc::new(WorkloadService {
            event_stream: Arc::clone(&event_stream) as Arc<dyn EventStream>,
            persistence: workload_persistence,

            template_service,
        });

        let workload: Workload = get_workload_fixture(None).into();
        workload_service
            .upsert(&workload, Some(operation_id))
            .await
            .unwrap();

        let target_persistence = MemoryPersistence::<Target>::default();
        let target_service = Arc::new(TargetService {
            persistence: Box::new(target_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let target: Target = get_target_fixture(None).into();
        let operation_id = target_service.upsert(&target, &None).await.unwrap();

        let deployment_persistence = Box::new(DeploymentMemoryPersistence::default());
        let deployment_service = Arc::new(DeploymentService {
            event_stream: Arc::clone(&event_stream) as Arc<dyn EventStream>,
            persistence: deployment_persistence,

            target_service,
        });

        let deployment: Deployment = get_deployment_fixture(None).into();

        deployment_service
            .upsert(&deployment, &Some(operation_id))
            .await
            .unwrap();

        let config_service = Arc::new(ConfigService {
            persistence: config_persistence,
            event_stream,

            deployment_service,
            workload_service,
        });

        let config_grpc_service = GrpcConfigService::new(Arc::clone(&config_service));

        let config = get_string_config_fixture();

        let request = Request::new(config.clone());

        let response = config_grpc_service
            .upsert(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(QueryConfigRequest {
            model_name: "deployment".to_string(),
            model_id: deployment.id.clone(),
        });

        let response = config_grpc_service
            .query(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.configs.len(), 1);

        let request = Request::new(ConfigIdRequest {
            config_id: config.id.clone(),
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
