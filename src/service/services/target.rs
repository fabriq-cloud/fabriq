use akira_core::{
    Event, EventStream, EventType, ModelType, OperationId, Persistence, TargetMessage,
};
use prost::Message;
use prost_types::Timestamp;
use std::{sync::Arc, time::SystemTime};

use crate::models::Target;

pub struct TargetService {
    pub persistence: Box<dyn Persistence<Target>>,
    pub event_stream: Arc<Box<dyn EventStream + 'static>>,
}

impl TargetService {
    pub fn create(
        &self,
        target: Target,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let target_id = self.persistence.create(target)?;

        let target = self.get_by_id(&target_id)?;
        let target = match target {
            Some(target) => target,
            None => return Err(anyhow::anyhow!("Couldn't find created target id returned")),
        };

        let operation_id = OperationId::unwrap_or_create(operation_id);
        let target_message: TargetMessage = target.into();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let create_target_event = Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Target as i32,
            serialized_model: target_message.encode_to_vec(),
            event_type: EventType::Created as i32,
            timestamp: Some(timestamp),
        };

        self.event_stream.send(&create_target_event)?;

        Ok(operation_id)
    }

    pub fn get_by_id(&self, target_id: &str) -> anyhow::Result<Option<Target>> {
        self.persistence.get_by_id(target_id)
    }

    pub fn delete(
        &self,
        target_id: &str,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let target = match self.get_by_id(target_id)? {
            Some(target) => target,
            None => return Err(anyhow::anyhow!("Target id {target_id} not found")),
        };

        let deleted_count = self.persistence.delete(target_id)?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Target id {target_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(&operation_id);
        let target_message: TargetMessage = target.into();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let delete_target_event = Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Target as i32,
            serialized_model: target_message.encode_to_vec(),
            event_type: EventType::Deleted as i32,
            timestamp: Some(timestamp),
        };

        self.event_stream.send(&delete_target_event)?;

        Ok(operation_id)
    }

    pub fn list(&self) -> anyhow::Result<Vec<Target>> {
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

        let new_target = Target {
            id: "eastus2".to_owned(),
            labels: vec!["location:eastus2".to_string()],
        };

        let target_persistence = MemoryPersistence::<Target>::default();

        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let target_service = TargetService {
            persistence: Box::new(target_persistence),
            event_stream,
        };

        let created_target_operation_id = target_service
            .create(new_target.clone(), &Some(OperationId::create()))
            .unwrap();
        assert_eq!(created_target_operation_id.id.len(), 36);

        let fetched_target = target_service.get_by_id(&new_target.id).unwrap().unwrap();
        assert_eq!(fetched_target.id, new_target.id);

        let deleted_target_operation_id = target_service.delete(&new_target.id, None).unwrap();
        assert_eq!(deleted_target_operation_id.id.len(), 36);
    }
}
