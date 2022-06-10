use akira_core::{
    DeploymentIdRequest, DeploymentMessage, DeploymentTrait, ListDeploymentsRequest,
    ListDeploymentsResponse, OperationId,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Deployment;
use crate::services::DeploymentService;

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
    async fn create(
        &self,
        request: Request<DeploymentMessage>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: Validate deployment id is valid

        let new_deployment = Deployment {
            id: request.get_ref().id.clone(),
            target_id: request.get_ref().target_id.clone(),
            workload_id: request.get_ref().workload_id.clone(),
            hosts: request.get_ref().hosts,
        };

        let operation_id = match self.service.create(new_deployment, &None).await {
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

    async fn delete(
        &self,
        request: Request<DeploymentIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: check that no workloads are currently still using deployment
        // Query workload service for workloads by deployment_id, error if any exist

        let operation_id = match self
            .service
            .delete(&request.into_inner().deployment_id, &None)
            .await
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

    async fn list(
        &self,
        _request: Request<ListDeploymentsRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let deployments = match self.service.list().await {
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
            .map(|deployment| DeploymentMessage {
                id: deployment.id.clone(),
                target_id: deployment.target_id.clone(),
                workload_id: deployment.workload_id.clone(),
                hosts: deployment.hosts,
            })
            .collect();

        let response = ListDeploymentsResponse {
            deployments: deployment_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use akira_core::{
        DeploymentIdRequest, DeploymentMessage, DeploymentTrait, EventStream,
        ListDeploymentsRequest,
    };
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcDeploymentService;

    use crate::models::Deployment;
    use crate::persistence::memory::MemoryPersistence;
    use crate::services::DeploymentService;

    #[tokio::test]
    async fn test_create_list_deployment() -> anyhow::Result<()> {
        let deployment_persistence =
            Box::new(MemoryPersistence::<Deployment, Deployment>::default());
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let deployment_service =
            Arc::new(DeploymentService::new(deployment_persistence, event_stream));

        let deployment_grpc_service = GrpcDeploymentService::new(Arc::clone(&deployment_service));

        let request = Request::new(DeploymentMessage {
            id: "deployment-grpc-test".to_string(),
            target_id: "target-fixture".to_string(),
            workload_id: "workload-fixture".to_string(),
            hosts: 2,
        });

        let response = deployment_grpc_service
            .create(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(ListDeploymentsRequest {});
        let _ = deployment_grpc_service
            .list(request)
            .await
            .unwrap()
            .into_inner();

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
