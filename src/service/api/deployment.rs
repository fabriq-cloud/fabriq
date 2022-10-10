use fabriq_core::{
    common::TemplateIdRequest, DeploymentIdRequest, DeploymentMessage, DeploymentTrait,
    ListDeploymentsRequest, ListDeploymentsResponse, OperationId, WorkloadIdRequest,
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
    #[tracing::instrument(name = "grpc::deployment::upsert")]
    async fn upsert(
        &self,
        request: Request<DeploymentMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let new_deployment: Deployment = request.into_inner().into();

        let operation_id = match self.service.upsert(&new_deployment, &None).await {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::InvalidArgument,
                    format!("upserting deployment failed with {}", err),
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

    #[tracing::instrument(name = "grpc::deployment::get_by_id")]
    async fn get_by_id(
        &self,
        request: Request<DeploymentIdRequest>,
    ) -> Result<Response<DeploymentMessage>, Status> {
        let deployment_id = request.into_inner().deployment_id;
        let deployment = match self.service.get_by_id(&deployment_id).await {
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
            .await
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
            .await
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
    use fabriq_core::{
        common::TemplateIdRequest,
        test::{get_deployment_fixture, get_target_fixture},
        DeploymentIdRequest, DeploymentTrait, EventStream, ListDeploymentsRequest,
        WorkloadIdRequest,
    };
    use fabriq_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcDeploymentService;

    use crate::models::Target;
    use crate::persistence::memory::{DeploymentMemoryPersistence, MemoryPersistence};
    use crate::services::{DeploymentService, TargetService};

    #[tokio::test]
    async fn test_create_list_deployment() -> anyhow::Result<()> {
        let deployment_persistence = Box::new(DeploymentMemoryPersistence::default());
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let target_persistence = MemoryPersistence::<Target>::default();
        let target_service = Arc::new(TargetService {
            persistence: Box::new(target_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let target: Target = get_target_fixture(None).into();
        target_service.upsert(&target, &None).await.unwrap();

        let deployment_service = Arc::new(DeploymentService {
            persistence: deployment_persistence,
            event_stream: Arc::clone(&event_stream),

            target_service,
        });

        let deployment_grpc_service = GrpcDeploymentService::new(Arc::clone(&deployment_service));

        let deployment = get_deployment_fixture(None);

        let request = Request::new(deployment.clone());

        let response = deployment_grpc_service
            .upsert(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(DeploymentIdRequest {
            deployment_id: deployment.id.clone(),
        });

        let response = deployment_grpc_service
            .get_by_id(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id, deployment.id);

        let request = Request::new(TemplateIdRequest {
            template_id: deployment.template_id.unwrap(),
        });

        let response = deployment_grpc_service
            .get_by_template_id(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.deployments.len(), 1);

        let request = Request::new(WorkloadIdRequest {
            workload_id: deployment.workload_id.clone(),
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
            deployment_id: deployment.id.to_string(),
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
