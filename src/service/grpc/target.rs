use fabriq_core::{
    ListTargetsRequest, ListTargetsResponse, OperationId, TargetIdRequest, TargetMessage,
    TargetTrait,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Target;
use crate::services::TargetService;

#[derive(Debug)]
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
    #[tracing::instrument(name = "grpc::target::upsert")]
    async fn upsert(
        &self,
        request: Request<TargetMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let new_target: Target = request.into_inner().into();

        let operation_id = match self.service.upsert(&new_target, &None).await {
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

    #[tracing::instrument(name = "grpc::target::delete")]
    async fn delete(
        &self,
        request: Request<TargetIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: check that no workloads are currently still using target
        // Query workload service for workloads by target_id, error if any exist

        let operation_id = match self
            .service
            .delete(&request.into_inner().target_id, None)
            .await
        {
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

    #[tracing::instrument(name = "grpc::target::get_by_id")]
    async fn get_by_id(
        &self,
        request: Request<TargetIdRequest>,
    ) -> Result<Response<TargetMessage>, Status> {
        let target_id = request.into_inner().target_id;
        let target = match self.service.get_by_id(&target_id).await {
            Ok(target) => target,
            Err(err) => {
                tracing::error!("get target with id {}: failed: {}", target_id, err);
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("get target with id {}: failed", &target_id),
                ));
            }
        };

        let target = match target {
            Some(target) => target,
            None => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("get target with id {}: not found", &target_id),
                ))
            }
        };

        let target_message: TargetMessage = target.into();

        Ok(Response::new(target_message))
    }

    #[tracing::instrument(name = "grpc::target::list")]
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
    use fabriq_core::{
        test::get_target_fixture, EventStream, ListTargetsRequest, TargetIdRequest, TargetTrait,
    };
    use fabriq_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcTargetService;

    use crate::models::Target;
    use crate::persistence::memory::MemoryPersistence;
    use crate::services::TargetService;

    #[tokio::test]
    async fn test_create_list_target() -> anyhow::Result<()> {
        let target_persistence = Box::new(MemoryPersistence::<Target>::default());
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let target_service = Arc::new(TargetService {
            persistence: target_persistence,
            event_stream,
        });

        let target_grpc_service = GrpcTargetService::new(Arc::clone(&target_service));

        let target = get_target_fixture(None);

        let request = Request::new(target.clone());

        let response = target_grpc_service
            .upsert(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(ListTargetsRequest {});
        let list_response = target_grpc_service
            .list(request)
            .await
            .unwrap()
            .into_inner();

        assert!(!list_response.targets.is_empty());

        let request = Request::new(TargetIdRequest {
            target_id: target.id.to_string(),
        });
        let get_response = target_grpc_service
            .get_by_id(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(get_response.id, target.id);

        let request = Request::new(TargetIdRequest {
            target_id: target.id.to_string(),
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
