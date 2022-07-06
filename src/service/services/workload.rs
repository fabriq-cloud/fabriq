use akira_core::{create_event, EventStream, EventType, ModelType, OperationId, WorkloadMessage};
use std::sync::Arc;

use crate::{models::Workload, persistence::WorkloadPersistence};

#[derive(Debug)]
pub struct WorkloadService {
    pub persistence: Box<dyn WorkloadPersistence>,
    pub event_stream: Arc<Box<dyn EventStream>>,
}

impl WorkloadService {
    #[tracing::instrument(name = "service::workload::create")]
    pub fn create(
        &self,
        workload: &Workload,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let workload_id = self.persistence.create(workload)?;

        let workload = self.get_by_id(&workload_id)?;
        let workload = match workload {
            Some(workload) => workload,
            None => {
                return Err(anyhow::anyhow!(
                    "Couldn't find created workload id returned"
                ))
            }
        };

        let operation_id = OperationId::unwrap_or_create(&operation_id);
        let create_event = create_event::<WorkloadMessage>(
            &None,
            &Some(workload.into()),
            EventType::Created,
            ModelType::Workload,
            &operation_id,
        );

        self.event_stream.send(&create_event)?;

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::workload::get_by_id")]
    pub fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Workload>> {
        self.persistence.get_by_id(host_id)
    }

    #[tracing::instrument(name = "service::workload::delete")]
    pub fn delete(
        &self,
        workload_id: &str,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let workload = match self.get_by_id(workload_id)? {
            Some(workload) => workload,
            None => return Err(anyhow::anyhow!("Workload id {workload_id} not found")),
        };

        let deleted_count = self.persistence.delete(workload_id)?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Workload id {workload_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(&operation_id);
        let delete_event = create_event::<WorkloadMessage>(
            &None,
            &Some(workload.into()),
            EventType::Deleted,
            ModelType::Workload,
            &operation_id,
        );

        self.event_stream.send(&delete_event)?;

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::workload::list")]
    pub fn list(&self) -> anyhow::Result<Vec<Workload>> {
        let results = self.persistence.list()?;

        Ok(results)
    }

    #[tracing::instrument(name = "service::workload::get_by_template_id")]
    pub fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Workload>> {
        self.persistence.get_by_template_id(template_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::memory::WorkloadMemoryPersistence;
    use akira_memory_stream::MemoryEventStream;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let new_workload = Workload {
            id: "cribbage-api".to_owned(),

            template_id: "external-service".to_owned(),
            workspace_id: "cribbage-team".to_owned(),
        };

        let workload_persistence = WorkloadMemoryPersistence::default();
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let workload_service = WorkloadService {
            persistence: Box::new(workload_persistence),
            event_stream,
        };

        let create_operation_id = workload_service
            .create(&new_workload, Some(OperationId::create()))
            .unwrap();
        assert_eq!(create_operation_id.id.len(), 36);

        let fetched_workload = workload_service
            .get_by_id(&new_workload.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_workload.id, new_workload.id);

        let delete_operation_id = workload_service.delete(&new_workload.id, None).unwrap();
        assert_eq!(delete_operation_id.id.len(), 36);
    }
}
