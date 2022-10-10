use tonic::{Request, Response, Status};

use crate::{
    common::{TemplateIdRequest, WorkloadIdRequest},
    test::get_workload_fixture,
    ListWorkloadsRequest, ListWorkloadsResponse, OperationId, WorkloadMessage, WorkloadTrait,
};

pub struct MockWorkloadClient {}

#[tonic::async_trait]
impl WorkloadTrait for MockWorkloadClient {
    async fn upsert(
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
        Ok(Response::new(get_workload_fixture(None)))
    }

    async fn get_by_template_id(
        &self,
        _request: Request<TemplateIdRequest>,
    ) -> Result<Response<ListWorkloadsResponse>, Status> {
        Ok(Response::new(ListWorkloadsResponse {
            workloads: vec![get_workload_fixture(None)],
        }))
    }

    async fn list(
        &self,
        _request: Request<ListWorkloadsRequest>,
    ) -> Result<Response<ListWorkloadsResponse>, Status> {
        Ok(Response::new(ListWorkloadsResponse {
            workloads: vec![get_workload_fixture(None)],
        }))
    }
}
