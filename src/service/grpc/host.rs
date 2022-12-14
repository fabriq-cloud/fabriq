use fabriq_core::{
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
    #[tracing::instrument(name = "grpc::host::upsert", skip_all)]
    async fn upsert(&self, request: Request<HostMessage>) -> Result<Response<OperationId>, Status> {
        let new_host: Host = request.into_inner().into();

        let operation_id = match self.service.upsert(&new_host, &None).await {
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

    #[tracing::instrument(name = "grpc::host::delete", skip_all)]
    async fn delete(
        &self,
        request: Request<DeleteHostRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: check that no workloads are currently still using host
        // Query workload service for workloads by host_id, error if any exist

        let operation_id = match self.service.delete(&request.into_inner().id, None).await {
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

    #[tracing::instrument(name = "grpc::host::list", skip_all)]
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
    use fabriq_core::{
        test::get_host_fixture, DeleteHostRequest, EventStream, HostTrait, ListHostsRequest,
    };
    use fabriq_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcHostService;

    use crate::persistence::memory::HostMemoryPersistence;
    use crate::services::HostService;

    #[tokio::test]
    async fn test_create_list_host() -> anyhow::Result<()> {
        let host_persistence = Box::<HostMemoryPersistence>::default();
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let host_service = Arc::new(HostService {
            persistence: host_persistence,
            event_stream,
        });

        let host_grpc_service = GrpcHostService::new(Arc::clone(&host_service));

        let host = get_host_fixture(None);

        let request = Request::new(host.clone());

        let response = host_grpc_service
            .upsert(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(ListHostsRequest {});
        let response = host_grpc_service.list(request).await.unwrap().into_inner();

        assert_eq!(response.hosts.len(), 1);

        let request = Request::new(DeleteHostRequest {
            id: host.id.to_string(),
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
