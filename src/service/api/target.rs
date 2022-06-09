use akira_core::{
    DeleteTargetRequest, ListTargetsRequest, ListTargetsResponse, OperationId, TargetMessage,
    TargetTrait,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Target;
use crate::services::TargetService;

pub struct GrpcTargetService {
    service: Arc<TargetService>,
}
impl GrpcTargetService {
    pub fn new(service: Arc<TargetService>) -> Self {
        GrpcTargetService { service }
    }
}

#[tonic::async_trait]
impl TargetTrait for GrpcTargetService {
    async fn create(
        &self,
        request: Request<TargetMessage>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: Validate target id is valid

        let new_target = Target {
            id: request.get_ref().id.clone(),
            labels: request.get_ref().labels.clone(),
        };

        let operation_id = match self.service.create(new_target, None).await {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::AlreadyExists,
                    format!("target {} already exists", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    async fn delete(
        &self,
        request: Request<DeleteTargetRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: check that no workloads are currently still using target
        // Query workload service for workloads by target_id, error if any exist

        let operation_id = match self.service.delete(&request.into_inner().id, None).await {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("target with id {} not found", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    async fn list(
        &self,
        _request: Request<ListTargetsRequest>,
    ) -> Result<Response<ListTargetsResponse>, Status> {
        let targets = match self.service.list().await {
            Ok(targets) => targets,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing targets failed with {}", err),
                ))
            }
        };

        println!("grpc service {:?}", targets);

        let target_messages = targets
            .iter()
            .map(|target| TargetMessage {
                id: target.id.clone(),
                labels: target.labels.clone(),
            })
            .collect();

        let response = ListTargetsResponse {
            targets: target_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use akira_core::{
        DeleteTargetRequest, EventStream, ListTargetsRequest, TargetMessage, TargetTrait,
    };
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcTargetService;

    use crate::models::Target;
    use crate::persistence::memory::MemoryPersistence;
    use crate::services::TargetService;

    #[tokio::test]
    async fn test_create_list_target() -> anyhow::Result<()> {
        let target_persistence = Box::new(MemoryPersistence::<Target, Target>::default());
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let target_service = Arc::new(TargetService::new(target_persistence, event_stream));

        let target_grpc_service = GrpcTargetService::new(Arc::clone(&target_service));

        let request = Request::new(TargetMessage {
            id: "target-grpc-test".to_string(),
            labels: vec!["region:eastus2".to_string()],
        });

        let response = target_grpc_service
            .create(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(ListTargetsRequest {});
        let _ = target_grpc_service
            .list(request)
            .await
            .unwrap()
            .into_inner();

        let request = Request::new(DeleteTargetRequest {
            id: "target-grpc-test".to_string(),
        });
        let response = target_grpc_service
            .delete(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        Ok(())
    }
}
