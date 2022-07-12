use akira_core::{create_event, EventStream, EventType, ModelType, OperationId, WorkspaceMessage};
use std::sync::Arc;

use crate::{models::Workspace, persistence::Persistence};

use super::WorkloadService;

#[derive(Debug)]
pub struct WorkspaceService {
    pub persistence: Box<dyn Persistence<Workspace>>,
    pub event_stream: Arc<dyn EventStream>,

    pub workload_service: Arc<WorkloadService>,
}

impl WorkspaceService {
    #[tracing::instrument(name = "service::workspace::create")]
    pub fn create(
        &self,
        workspace: &Workspace,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        // TODO: Use an Error enumeration to return specific error

        match self.get_by_id(&workspace.id)? {
            Some(workspace) => {
                return Err(anyhow::anyhow!(
                    "Deployment id {} already exists",
                    workspace.id
                ))
            }
            None => {}
        };

        let workspace_id = self.persistence.create(workspace)?;

        let workspace = self.get_by_id(&workspace_id)?;
        let workspace = match workspace {
            Some(workspace) => workspace,
            None => {
                return Err(anyhow::anyhow!(
                    "Created workspace id {} not found",
                    workspace_id
                ))
            }
        };

        let operation_id = OperationId::unwrap_or_create(operation_id);
        let create_event = create_event::<WorkspaceMessage>(
            &None,
            &Some(workspace.clone().into()),
            EventType::Created,
            ModelType::Workspace,
            &operation_id,
        );

        self.event_stream.send(&create_event)?;

        tracing::info!("workspace created: {:?}", workspace);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::workspace::get_by_id")]
    pub fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Workspace>> {
        self.persistence.get_by_id(host_id)
    }

    #[tracing::instrument(name = "service::workspace::delete")]
    pub fn delete(
        &self,
        workspace_id: &str,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let workspace = match self.get_by_id(workspace_id)? {
            Some(workspace) => workspace,
            None => return Err(anyhow::anyhow!("deployment id {workspace_id} not found")),
        };

        // Check if there are any workloads associated with this workspace, fail if so.
        /*

        let workloads = self
            .workload_service
            .get_by_workspace_id(workspace_id)
            .await?;

        */

        let deleted_count = self.persistence.delete(workspace_id)?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("workspace id {workspace_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(&operation_id);
        let delete_event = create_event::<WorkspaceMessage>(
            &Some(workspace.clone().into()),
            &None,
            EventType::Deleted,
            ModelType::Workspace,
            &operation_id,
        );

        self.event_stream.send(&delete_event)?;

        tracing::info!("workspace deleted: {:?}", workspace);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::workspace::list")]
    pub fn list(&self) -> anyhow::Result<Vec<Workspace>> {
        let results = self.persistence.list()?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use akira_memory_stream::MemoryEventStream;

    use super::*;
    use crate::{
        persistence::memory::{MemoryPersistence, WorkloadMemoryPersistence},
        services::WorkloadService,
    };

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let workload_persistence = WorkloadMemoryPersistence::default();
        let workload_service = Arc::new(WorkloadService {
            persistence: Box::new(workload_persistence),
            event_stream: event_stream.clone(),
        });

        let workspace_persistence = MemoryPersistence::<Workspace>::default();
        let workspace_service = WorkspaceService {
            persistence: Box::new(workspace_persistence),
            event_stream,

            workload_service,
        };

        let new_workspace = Workspace {
            id: "workspace-under-test".to_owned(),
        };

        let create_operation_id = workspace_service.create(&new_workspace, &None).unwrap();
        assert_eq!(create_operation_id.id.len(), 36);

        let fetched_workspace = workspace_service
            .get_by_id(&new_workspace.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_workspace.id, new_workspace.id);

        let all_workspaces = workspace_service.list().unwrap();
        assert_eq!(all_workspaces.len(), 1);

        let delete_operation_id = workspace_service
            .delete(&new_workspace.id, Some(OperationId::create()))
            .unwrap();
        assert_eq!(delete_operation_id.id.len(), 36);
    }
}
