use akira_core::{Event, EventStream, EventType, ModelType, OperationId, WorkloadMessage};
use prost::Message;
use prost_types::Timestamp;
use std::{sync::Arc, time::SystemTime};

use crate::{models::Workload, persistence::Persistence};

pub struct WorkloadService {
    pub persistence: Box<dyn Persistence<Workload>>,
    pub event_stream: Arc<Box<dyn EventStream + 'static>>,
}

impl WorkloadService {
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
        let workload_message: WorkloadMessage = workload.into();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let create_workload_event = Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Workload as i32,
            serialized_model: workload_message.encode_to_vec(),
            event_type: EventType::Created as i32,
            timestamp: Some(timestamp),
        };

        self.event_stream.send(&create_workload_event)?;

        Ok(operation_id)
    }

    pub fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Workload>> {
        self.persistence.get_by_id(host_id)
    }

    /*
    pub async fn get_by_workspace_id(&self, workspace_id: &str) -> anyhow::Result<Vec<Workload>> {
        self.persistence.get_by_workspace_id(workspace_id).await
    }
    */

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
        let workload_message: WorkloadMessage = workload.into();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let delete_workload_event = Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Workload as i32,
            serialized_model: workload_message.encode_to_vec(),
            event_type: EventType::Deleted as i32,
            timestamp: Some(timestamp),
        };

        self.event_stream.send(&delete_workload_event)?;

        Ok(operation_id)
    }

    pub fn list(&self) -> anyhow::Result<Vec<Workload>> {
        let results = self.persistence.list()?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use akira_memory_stream::MemoryEventStream;
    use dotenv::dotenv;

    use super::*;
    use crate::persistence::memory::MemoryPersistence;

    #[test]
    fn test_create_get_delete() {
        dotenv().ok();

        let new_workload = Workload {
            id: "cribbage-api".to_owned(),

            template_id: "external-service".to_owned(),
            workspace_id: "cribbage-team".to_owned(),
        };

        let workload_persistence = MemoryPersistence::<Workload>::default();
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
