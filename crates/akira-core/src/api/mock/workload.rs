use tonic::{Request, Response, Status};

use crate::{
    common::{TemplateIdRequest, WorkloadIdRequest},
    ListWorkloadsRequest, ListWorkloadsResponse, OperationId, WorkloadMessage, WorkloadTrait,
};

pub struct MockWorkloadClient {}

#[tonic::async_trait]
impl WorkloadTrait for MockWorkloadClient {
    async fn create(
        &self,
        _request: Request<WorkloadMessage>,
    ) -> Result<Response<OperationId>, Status> {
        Ok(Response::new(OperationId::create()))
    }

    async fn delete(
        &self,
        _request: Request<WorkloadIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        Ok(Response::new(OperationId::create()))
    }

    async fn get_by_id(
        &self,
        _request: Request<WorkloadIdRequest>,
    ) -> Result<Response<WorkloadMessage>, Status> {
        Ok(Response::new(WorkloadMessage {
            id: "workload-fixture".to_owned(),
            template_id: "template-fixture".to_owned(),
            workspace_id: "workspace-fixture".to_owned(),
        }))
    }

    async fn get_by_template_id(
        &self,
        _request: Request<TemplateIdRequest>,
    ) -> Result<Response<ListWorkloadsResponse>, Status> {
        let workload = WorkloadMessage {
            id: "workload-fixture".to_owned(),
            template_id: "template-fixture".to_owned(),
            workspace_id: "workspace-fixture".to_owned(),
        };

        Ok(Response::new(ListWorkloadsResponse {
            workloads: vec![workload],
        }))
    }

    async fn list(
        &self,
        _request: Request<ListWorkloadsRequest>,
    ) -> Result<Response<ListWorkloadsResponse>, Status> {
        let workload = WorkloadMessage {
            id: "workload-fixture".to_owned(),
            template_id: "template-fixture".to_owned(),
            workspace_id: "workspace-fixture".to_owned(),
        };

        Ok(Response::new(ListWorkloadsResponse {
            workloads: vec![workload],
        }))
    }
}
