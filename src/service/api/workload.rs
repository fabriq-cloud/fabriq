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
    #[tracing::instrument(name = "grpc::workload::upsert")]
    async fn upsert(
        &self,
        request: Request<WorkloadMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let new_workload: Workload = request.into_inner().into();

        let operation_id = match self.service.upsert(&new_workload, None).await {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::InvalidArgument,
                    format!("upserting workload failed with {}", err),
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

        let operation_id = match self
            .service
            .delete(&request.into_inner().workload_id, None)
            .await
        {
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
        let workload = match self.service.get_by_id(&workload_id).await {
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
            .await
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
        let workloads = match self.service.list().await {
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
    use akira_core::test::{get_template_fixture, get_workload_fixture};
    use akira_core::{EventStream, ListWorkloadsRequest, WorkloadIdRequest, WorkloadTrait};
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcWorkloadService;

    use crate::models::Template;
    use crate::persistence::memory::{MemoryPersistence, WorkloadMemoryPersistence};
    use crate::services::{TemplateService, WorkloadService};

    #[tokio::test]
    async fn test_create_list_workload() -> anyhow::Result<()> {
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let template_persistence = MemoryPersistence::<Template>::default();
        let template_service = Arc::new(TemplateService {
            persistence: Box::new(template_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let template: Template = get_template_fixture(Some("template-fixture")).into();
        template_service.upsert(&template, None).await.unwrap();

        let workload_persistence = Box::new(WorkloadMemoryPersistence::default());
        let workload_service = Arc::new(WorkloadService {
            persistence: workload_persistence,
            event_stream,

            template_service,
        });

        let workload_grpc_service = GrpcWorkloadService::new(Arc::clone(&workload_service));
        let workload = get_workload_fixture(None);

        let request = Request::new(workload.clone());

        let create_response = workload_grpc_service
            .upsert(request)
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
