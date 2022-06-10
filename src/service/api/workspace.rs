use akira_core::{
    DeleteWorkspaceRequest, ListWorkspacesRequest, ListWorkspacesResponse, OperationId,
    WorkspaceMessage, WorkspaceTrait,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Workspace;
use crate::services::WorkspaceService;

pub struct GrpcWorkspaceService {
    service: Arc<WorkspaceService>,
}
impl GrpcWorkspaceService {
    pub fn new(service: Arc<WorkspaceService>) -> Self {
        GrpcWorkspaceService { service }
    }
}

#[tonic::async_trait]
impl WorkspaceTrait for GrpcWorkspaceService {
    async fn create(
        &self,
        request: Request<WorkspaceMessage>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: Validate workspace id is valid

        let new_workspace: Workspace = Workspace {
            id: request.get_ref().id.clone(),
        };

        let operation_id = match self.service.create(new_workspace, &None).await {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::AlreadyExists,
                    format!("workspace {} already exists", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    async fn delete(
        &self,
        request: Request<DeleteWorkspaceRequest>,
    ) -> Result<Response<OperationId>, Status> {
        let operation_id = match self.service.delete(&request.into_inner().id, None).await {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("workspace with id {} not found", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    async fn list(
        &self,
        _request: Request<ListWorkspacesRequest>,
    ) -> Result<Response<ListWorkspacesResponse>, Status> {
        let workspaces = match self.service.list().await {
            Ok(workspaces) => workspaces,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing workspaces failed with {}", err),
                ))
            }
        };

        println!("grpc service {:?}", workspaces);

        let workspace_messages = workspaces
            .iter()
            .map(|workspace| WorkspaceMessage {
                id: workspace.id.clone(),
            })
            .collect();

        let response = ListWorkspacesResponse {
            workspaces: workspace_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use akira_core::{DeleteWorkspaceRequest, EventStream, ListWorkspacesRequest, WorkspaceTrait};
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::{GrpcWorkspaceService, WorkspaceMessage};

    use crate::models::{Workload, Workspace};
    use crate::persistence::memory::MemoryPersistence;
    use crate::services::{WorkloadService, WorkspaceService};

    #[tokio::test]
    async fn test_create_list_workspace() -> anyhow::Result<()> {
        let workspace_persistence = Box::new(MemoryPersistence::<Workspace, Workspace>::default());
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let workload_persistence = MemoryPersistence::<Workload, Workload>::default();
        let workload_service = Arc::new(WorkloadService {
            persistence: Box::new(workload_persistence),
            event_stream: event_stream.clone(),
        });

        let workspace_service = Arc::new(WorkspaceService {
            persistence: workspace_persistence,
            event_stream,

            workload_service,
        });

        let workspace_grpc_service = GrpcWorkspaceService::new(Arc::clone(&workspace_service));

        let request = Request::new(WorkspaceMessage {
            id: "climate-api-team".to_string(),
        });

        let response = workspace_grpc_service
            .create(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(ListWorkspacesRequest {});
        let _ = workspace_grpc_service
            .list(request)
            .await
            .unwrap()
            .into_inner();

        let request = Request::new(DeleteWorkspaceRequest {
            id: "climate-api-team".to_string(),
        });
        let response = workspace_grpc_service
            .delete(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        Ok(())
    }
}
