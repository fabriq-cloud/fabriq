use tonic::{Request, Response, Status};

use crate::{
    common::{DeploymentIdRequest, TemplateIdRequest},
    DeploymentMessage, DeploymentTrait, ListDeploymentsRequest, ListDeploymentsResponse,
    OperationId, WorkloadIdRequest,
};

pub struct MockDeploymentClient {}

#[tonic::async_trait]
impl DeploymentTrait for MockDeploymentClient {
    async fn create(
        &self,
        _request: Request<DeploymentMessage>,
    ) -> Result<Response<OperationId>, Status> {
        Ok(Response::new(OperationId::create()))
    }

    async fn delete(
        &self,
        _request: Request<DeploymentIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        Ok(Response::new(OperationId::create()))
    }

    async fn get_by_id(
        &self,
        _request: Request<DeploymentIdRequest>,
    ) -> Result<Response<DeploymentMessage>, Status> {
        Ok(Response::new(DeploymentMessage {
            id: "deployment-fixture".to_owned(),
            template_id: Some("template-fixture".to_owned()),
            target_id: "target-fixture".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            host_count: 2,
        }))
    }

    async fn get_by_template_id(
        &self,
        _request: Request<TemplateIdRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let deployment = DeploymentMessage {
            id: "deployment-fixture".to_owned(),
            template_id: Some("template-fixture".to_owned()),
            target_id: "target-fixture".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            host_count: 2,
        };

        Ok(Response::new(ListDeploymentsResponse {
            deployments: vec![deployment],
        }))
    }

    async fn get_by_workload_id(
        &self,
        _request: Request<WorkloadIdRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let deployment = DeploymentMessage {
            id: "deployment-fixture".to_owned(),
            template_id: Some("template-fixture".to_owned()),
            target_id: "target-fixture".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            host_count: 2,
        };

        Ok(Response::new(ListDeploymentsResponse {
            deployments: vec![deployment],
        }))
    }

    async fn list(
        &self,
        _request: Request<ListDeploymentsRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let deployment = DeploymentMessage {
            id: "deployment-fixture".to_owned(),
            template_id: Some("template-fixture".to_owned()),
            target_id: "target-fixture".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            host_count: 2,
        };

        Ok(Response::new(ListDeploymentsResponse {
            deployments: vec![deployment],
        }))
    }
}
