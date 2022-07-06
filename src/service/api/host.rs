use akira_core::{
    DeleteHostRequest, HostMessage, HostTrait, ListHostsRequest, ListHostsResponse, OperationId,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Host;
use crate::services::HostService;

#[derive(Debug)]
pub struct GrpcHostService {
    service: Arc<HostService>,
}

impl GrpcHostService {
    pub fn new(service: Arc<HostService>) -> Self {
        GrpcHostService { service }
    }
}

#[tonic::async_trait]
impl HostTrait for GrpcHostService {
    #[tracing::instrument(name = "grpc::host::create")]
    async fn create(&self, request: Request<HostMessage>) -> Result<Response<OperationId>, Status> {
        let new_host: Host = request.into_inner().into();

        let operation_id = match self.service.create(&new_host, &None) {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::AlreadyExists,
                    format!("host {} already exists", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    #[tracing::instrument(name = "grpc::host::delete")]
    async fn delete(
        &self,
        request: Request<DeleteHostRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: check that no workloads are currently still using host
        // Query workload service for workloads by host_id, error if any exist

        let operation_id = match self.service.delete(&request.into_inner().id, None) {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("host with id {} not found", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    #[tracing::instrument(name = "grpc::host::list")]
    async fn list(
        &self,
        _request: Request<ListHostsRequest>,
    ) -> Result<Response<ListHostsResponse>, Status> {
        let hosts = match self.service.list().await {
            Ok(hosts) => hosts,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing hosts failed with {}", err),
                ))
            }
        };

        let host_messages = hosts
            .iter()
            .map(|host| HostMessage {
                id: host.id.clone(),
                labels: host.labels.clone(),
            })
            .collect();

        let response = ListHostsResponse {
            hosts: host_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use akira_core::{DeleteHostRequest, EventStream, HostMessage, HostTrait, ListHostsRequest};
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcHostService;

    use crate::persistence::memory::HostMemoryPersistence;
    use crate::services::HostService;

    #[tokio::test]
    async fn test_create_list_host() -> anyhow::Result<()> {
        let host_persistence = Box::new(HostMemoryPersistence::default());
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let host_service = Arc::new(HostService {
            persistence: host_persistence,
            event_stream,
        });

        let host_grpc_service = GrpcHostService::new(Arc::clone(&host_service));

        let request = Request::new(HostMessage {
            id: "host-grpc-test".to_string(),
            labels: vec!["region:eastus2".to_string(), "cloud:azure".to_string()],
        });

        let response = host_grpc_service
            .create(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(ListHostsRequest {});
        let _ = host_grpc_service.list(request).await.unwrap().into_inner();

        let request = Request::new(DeleteHostRequest {
            id: "host-grpc-test".to_string(),
        });
        let response = host_grpc_service
            .delete(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        Ok(())
    }
}
