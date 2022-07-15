use akira_core::common::TemplateIdRequest;
use akira_core::{
    ListWorkloadsRequest, ListWorkloadsResponse, OperationId, WorkloadIdRequest, WorkloadMessage,
    WorkloadTrait,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Workload;
use crate::services::WorkloadService;

#[derive(Debug)]
pub struct GrpcWorkloadService {
    service: Arc<WorkloadService>,
}
impl GrpcWorkloadService {
    pub fn new(service: Arc<WorkloadService>) -> Self {
        GrpcWorkloadService { service }
    }
}

#[tonic::async_trait]
impl WorkloadTrait for GrpcWorkloadService {
    #[tracing::instrument(name = "grpc::workload::create")]
    async fn create(
        &self,
        request: Request<WorkloadMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let new_workload: Workload = request.into_inner().into();

        let operation_id = match self.service.create(&new_workload, None) {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("creating workload failed with {}", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    #[tracing::instrument(name = "grpc::workload::delete")]
    async fn delete(
        &self,
        request: Request<WorkloadIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: Check that no workloads are currently still using workload
        // Query workload service for workloads by workload_id

        let operation_id = match self.service.delete(&request.into_inner().workload_id, None) {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("deleting workspace failed with {}", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    #[tracing::instrument(name = "grpc::workload::get_by_id")]
    async fn get_by_id(
        &self,
        request: Request<WorkloadIdRequest>,
    ) -> Result<Response<WorkloadMessage>, Status> {
        let workload_id = request.into_inner().workload_id;
        let workload = match self.service.get_by_id(&workload_id) {
            Ok(workload) => workload,
            Err(err) => {
                tracing::error!("get target with id {}: failed: {}", workload_id, err);
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("get target with id {}: failed", &workload_id),
                ));
            }
        };

        let workload = match workload {
            Some(workload) => workload,
            None => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("get workload with id {}: not found", &workload_id),
                ))
            }
        };

        let workload_message: WorkloadMessage = workload.into();

        Ok(Response::new(workload_message))
    }

    #[tracing::instrument(name = "grpc::deployment::get_by_template_id")]
    async fn get_by_template_id(
        &self,
        request: Request<TemplateIdRequest>,
    ) -> Result<Response<ListWorkloadsResponse>, Status> {
        let workloads = match self
            .service
            .get_by_template_id(&request.into_inner().template_id)
        {
            Ok(workloads) => workloads,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing workloads failed with {}", err),
                ))
            }
        };

        let workload_messages = workloads
            .iter()
            .map(|workload| workload.clone().into())
            .collect();

        let response = ListWorkloadsResponse {
            workloads: workload_messages,
        };

        Ok(Response::new(response))
    }

    #[tracing::instrument(name = "grpc::workload::list")]
    async fn list(
        &self,
        _request: Request<ListWorkloadsRequest>,
    ) -> Result<Response<ListWorkloadsResponse>, Status> {
        let workloads = match self.service.list() {
            Ok(workloads) => workloads,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing workloads failed with {}", err),
                ))
            }
        };

        let workload_messages = workloads
            .iter()
            .map(|workload| workload.clone().into())
            .collect();

        let response = ListWorkloadsResponse {
            workloads: workload_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use akira_core::common::TemplateIdRequest;
    use akira_core::test::get_workload_fixture;
    use akira_core::{EventStream, ListWorkloadsRequest, WorkloadIdRequest, WorkloadTrait};
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcWorkloadService;

    use crate::persistence::memory::WorkloadMemoryPersistence;
    use crate::services::WorkloadService;

    #[tokio::test]
    async fn test_create_list_workload() -> anyhow::Result<()> {
        let workload_persistence = Box::new(WorkloadMemoryPersistence::default());
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let workload_service = Arc::new(WorkloadService {
            persistence: workload_persistence,
            event_stream,
        });

        let workload_grpc_service = GrpcWorkloadService::new(Arc::clone(&workload_service));
        let workload = get_workload_fixture(None);

        let request = Request::new(workload.clone());

        let create_response = workload_grpc_service
            .create(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(create_response.id.len(), 36);

        let request = Request::new(TemplateIdRequest {
            template_id: workload.template_id,
        });

        let response = workload_grpc_service
            .get_by_template_id(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.workloads.len(), 1);

        let request = Request::new(ListWorkloadsRequest {});

        let list_response = workload_grpc_service
            .list(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(list_response.workloads.len(), 1);

        let request = Request::new(WorkloadIdRequest {
            workload_id: workload.id.clone(),
        });

        let get_by_id_response = workload_grpc_service
            .get_by_id(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(get_by_id_response.id, workload.id);

        let request = Request::new(WorkloadIdRequest {
            workload_id: workload.id,
        });

        let delete_response = workload_grpc_service
            .delete(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(delete_response.id.len(), 36);

        Ok(())
    }
}
