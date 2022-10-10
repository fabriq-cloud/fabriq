use tonic::{Request, Response, Status};

use crate::{
    common::{DeploymentIdRequest, TemplateIdRequest},
    DeploymentMessage, DeploymentTrait, ListDeploymentsRequest, ListDeploymentsResponse,
    OperationId, WorkloadIdRequest, WorkloadMessage,
};

pub struct MockDeploymentClient {}

impl MockDeploymentClient {
    pub fn get_deployment_fixture() -> DeploymentMessage {
        let workspace_id = "workspace-fixture";
        let workload_name = "workload-fixture";
        let template_id = "template-fixture";
        let target_id = "target-fixture";
        let workload_id = WorkloadMessage::make_id(workspace_id, workload_name);
        let deployment_name = "deployment-fixture";
        let deployment_id = DeploymentMessage::make_id(&workload_id, deployment_name);

        DeploymentMessage {
            id: deployment_id,
            name: deployment_name.to_owned(),
            workload_id,
            target_id: target_id.to_string(),
            template_id: Some(template_id.to_string()),
            host_count: 2,
        }
    }
}

#[tonic::async_trait]
impl DeploymentTrait for MockDeploymentClient {
    async fn upsert(
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
        Ok(Response::new(MockDeploymentClient::get_deployment_fixture()))
    }

    async fn get_by_template_id(
        &self,
        _request: Request<TemplateIdRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        Ok(Response::new(ListDeploymentsResponse {
            deployments: vec![MockDeploymentClient::get_deployment_fixture()],
        }))
    }

    async fn get_by_workload_id(
        &self,
        _request: Request<WorkloadIdRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        Ok(Response::new(ListDeploymentsResponse {
            deployments: vec![MockDeploymentClient::get_deployment_fixture()],
        }))
    }

    async fn list(
        &self,
        _request: Request<ListDeploymentsRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        Ok(Response::new(ListDeploymentsResponse {
            deployments: vec![MockDeploymentClient::get_deployment_fixture()],
        }))
    }
}
