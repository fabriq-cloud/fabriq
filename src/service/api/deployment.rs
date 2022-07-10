use akira_core::common::TemplateIdRequest;
use akira_core::{
    DeploymentIdRequest, DeploymentMessage, DeploymentTrait, ListDeploymentsRequest,
    ListDeploymentsResponse, OperationId, WorkloadIdRequest,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Deployment;
use crate::services::DeploymentService;

#[derive(Debug)]
pub struct GrpcDeploymentService {
    service: Arc<DeploymentService>,
}

impl GrpcDeploymentService {
    pub fn new(service: Arc<DeploymentService>) -> Self {
        GrpcDeploymentService { service }
    }
}

#[tonic::async_trait]
impl DeploymentTrait for GrpcDeploymentService {
    #[tracing::instrument(name = "grpc::deployment::create")]
    async fn create(
        &self,
        request: Request<DeploymentMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let new_deployment: Deployment = request.into_inner().into();

        let operation_id = match self.service.create(&new_deployment, &None) {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::AlreadyExists,
                    format!("deployment {} already exists", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    #[tracing::instrument(name = "grpc::deployment::delete")]
    async fn delete(
        &self,
        request: Request<DeploymentIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: check that no workloads are currently still using deployment
        // Query workload service for workloads by deployment_id, error if any exist

        let operation_id = match self
            .service
            .delete(&request.into_inner().deployment_id, &None)
        {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("deployment with id {} not found", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    #[tracing::instrument(name = "grpc::deployment::get_by_id")]
    async fn get_by_id(
        &self,
        request: Request<DeploymentIdRequest>,
    ) -> Result<Response<DeploymentMessage>, Status> {
        let deployment_id = request.into_inner().deployment_id;
        let deployment = match self.service.get_by_id(&deployment_id) {
            Ok(deployment) => deployment,
            Err(err) => {
                tracing::error!("get target with id {}: failed: {}", deployment_id, err);
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("get target with id {}: failed", &deployment_id),
                ));
            }
        };

        let deployment = match deployment {
            Some(deployment) => deployment,
            None => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("get workload with id {}: not found", &deployment_id),
                ))
            }
        };

        let deployment_message: DeploymentMessage = deployment.into();

        Ok(Response::new(deployment_message))
    }

    #[tracing::instrument(name = "grpc::deployment::get_by_template_id")]
    async fn get_by_template_id(
        &self,
        request: Request<TemplateIdRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let deployments = match self
            .service
            .get_by_template_id(&request.into_inner().template_id)
        {
            Ok(deployments) => deployments,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing deployments failed with {}", err),
                ))
            }
        };

        let deployment_messages = deployments
            .iter()
            .map(|deployment| deployment.clone().into())
            .collect();

        let response = ListDeploymentsResponse {
            deployments: deployment_messages,
        };

        Ok(Response::new(response))
    }

    #[tracing::instrument(name = "grpc::deployment::get_by_template_id")]
    async fn get_by_workload_id(
        &self,
        request: Request<WorkloadIdRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let deployments = match self
            .service
            .get_by_workload_id(&request.into_inner().workload_id)
        {
            Ok(deployments) => deployments,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing deployments failed with {}", err),
                ))
            }
        };

        let deployment_messages = deployments
            .iter()
            .map(|deployment| deployment.clone().into())
            .collect();

        let response = ListDeploymentsResponse {
            deployments: deployment_messages,
        };

        Ok(Response::new(response))
    }

    #[tracing::instrument(name = "grpc::deployment::list")]
    async fn list(
        &self,
        _request: Request<ListDeploymentsRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let deployments = match self.service.list() {
            Ok(deployments) => deployments,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing deployments failed with {}", err),
                ))
            }
        };

        let deployment_messages = deployments
            .iter()
            .map(|deployment| deployment.clone().into())
            .collect();

        let response = ListDeploymentsResponse {
            deployments: deployment_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use akira_core::common::TemplateIdRequest;
    use akira_core::{
        DeploymentIdRequest, DeploymentMessage, DeploymentTrait, EventStream,
        ListDeploymentsRequest, WorkloadIdRequest,
    };
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcDeploymentService;

    use crate::persistence::memory::DeploymentMemoryPersistence;
    use crate::services::DeploymentService;

    #[tokio::test]
    async fn test_create_list_deployment() -> anyhow::Result<()> {
        let deployment_persistence = Box::new(DeploymentMemoryPersistence::default());
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let deployment_service = Arc::new(DeploymentService {
            persistence: deployment_persistence,
            event_stream,
        });

        let deployment_grpc_service = GrpcDeploymentService::new(Arc::clone(&deployment_service));

        let request = Request::new(DeploymentMessage {
            id: "deployment-grpc-test".to_string(),
            target_id: "target-fixture".to_string(),
            workload_id: "workload-fixture".to_string(),
            template_id: Some("external-service".to_string()),
            host_count: 2,
        });

        let response = deployment_grpc_service
            .create(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(DeploymentIdRequest {
            deployment_id: "deployment-grpc-test".to_string(),
        });

        let response = deployment_grpc_service
            .get_by_id(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id, "deployment-grpc-test");

        let request = Request::new(TemplateIdRequest {
            template_id: "external-service".to_string(),
        });

        let response = deployment_grpc_service
            .get_by_template_id(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.deployments.len(), 1);

        let request = Request::new(WorkloadIdRequest {
            workload_id: "workload-fixture".to_string(),
        });

        let response = deployment_grpc_service
            .get_by_workload_id(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.deployments.len(), 1);

        let request = Request::new(ListDeploymentsRequest {});
        let response = deployment_grpc_service
            .list(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.deployments.len(), 1);

        let request = Request::new(DeploymentIdRequest {
            deployment_id: "deployment-grpc-test".to_string(),
        });
        let response = deployment_grpc_service
            .delete(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        Ok(())
    }
}
