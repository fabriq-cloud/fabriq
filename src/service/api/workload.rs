use akira_core::{
    DeleteWorkloadRequest, ListWorkloadsRequest, ListWorkloadsResponse, OperationId,
    WorkloadMessage, WorkloadTrait,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Workload;
use crate::services::WorkloadService;

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
    async fn create(
        &self,
        request: Request<WorkloadMessage>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: Validate NewWorkloadMessage to be valid

        let new_workload = Workload {
            id: request.get_ref().id.clone(),

            template_id: request.get_ref().template_id.clone(),
            workspace_id: request.get_ref().workspace_id.clone(),
        };

        let operation_id = match self.service.create(new_workload, None) {
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

    async fn delete(
        &self,
        request: Request<DeleteWorkloadRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: Check that no workloads are currently still using workload
        // Query workload service for workloads by workload_id

        let operation_id = match self.service.delete(&request.into_inner().id, None) {
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
            .map(|workload| WorkloadMessage {
                id: workload.id.clone(),
                template_id: workload.template_id.clone(),
                workspace_id: workload.workspace_id.clone(),
            })
            .collect();

        let response = ListWorkloadsResponse {
            workloads: workload_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use akira_core::{DeleteWorkloadRequest, EventStream, ListWorkloadsRequest, WorkloadTrait};
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::{GrpcWorkloadService, WorkloadMessage};

    use crate::models::Workload;
    use crate::persistence::memory::MemoryPersistence;
    use crate::services::WorkloadService;

    #[tokio::test]
    async fn test_create_list_workload() -> anyhow::Result<()> {
        let workload_persistence = Box::new(MemoryPersistence::<Workload>::default());
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let workload_service = Arc::new(WorkloadService {
            persistence: workload_persistence,
            event_stream,
        });

        let workload_grpc_service = GrpcWorkloadService::new(Arc::clone(&workload_service));

        let request = Request::new(WorkloadMessage {
            id: "cribbage-api".to_owned(),
            template_id: "external-service".to_owned(),
            workspace_id: "cribbage-api-team".to_owned(),
        });

        let create_response = workload_grpc_service
            .create(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(create_response.id.len(), 36);

        let request = Request::new(ListWorkloadsRequest {});

        let list_response = workload_grpc_service
            .list(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(list_response.workloads.len(), 1);

        let request = Request::new(DeleteWorkloadRequest {
            id: "cribbage-api".to_owned(),
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
