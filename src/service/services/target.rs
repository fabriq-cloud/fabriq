use akira_core::{Event, EventStream, EventType, ModelType, OperationId, TargetMessage};
use prost::Message;
use prost_types::Timestamp;
use std::{sync::Arc, time::SystemTime};

use crate::{
    models::{Host, Target},
    persistence::Persistence,
};

pub struct TargetService {
    pub persistence: Box<dyn Persistence<Target>>,
    pub event_stream: Arc<Box<dyn EventStream>>,
}

impl TargetService {
    pub fn serialize_model(model: &Option<Target>) -> Option<Vec<u8>> {
        match model {
            Some(assignment) => {
                let message: TargetMessage = assignment.clone().into();
                Some(message.encode_to_vec())
            }
            None => None,
        }
    }

    pub fn create_event(
        previous_model: &Option<Target>,
        current_model: &Option<Target>,
        event_type: EventType,
        operation_id: &OperationId,
    ) -> Event {
        let serialized_previous_model = Self::serialize_model(previous_model);
        let serialized_current_model = Self::serialize_model(current_model);

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Target as i32,
            serialized_previous_model,
            serialized_current_model,
            event_type: event_type as i32,
            timestamp: Some(timestamp),
        }
    }

    pub fn create(
        &self,
        target: &Target,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let target_id = self.persistence.create(target)?;

        let target = self.get_by_id(&target_id)?;
        let target = match target {
            Some(target) => target,
            None => return Err(anyhow::anyhow!("Couldn't find created target id returned")),
        };

        let operation_id = OperationId::unwrap_or_create(operation_id);
        let create_event =
            Self::create_event(&None, &Some(target), EventType::Created, &operation_id);

        self.event_stream.send(&create_event)?;

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
        let delete_event =
            Self::create_event(&Some(target), &None, EventType::Deleted, &operation_id);

        self.event_stream.send(&delete_event)?;

        Ok(operation_id)
    }

    pub fn list(&self) -> anyhow::Result<Vec<Target>> {
        let results = self.persistence.list()?;

        Ok(results)
    }

    pub fn get_matching_host(&self, host: &Host) -> anyhow::Result<Vec<Target>> {
        // TODO: Naive implementation, use proper query
        let targets = self.list()?;
        let targets_matching_host = targets
            .iter()
            .filter(|target| {
                for label in &target.labels {
                    if !host.labels.contains(label) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect::<Vec<_>>();

        Ok(targets_matching_host)
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
            .create(&new_target, &Some(OperationId::create()))
            .unwrap();
        assert_eq!(created_target_operation_id.id.len(), 36);

        let fetched_target = target_service.get_by_id(&new_target.id).unwrap().unwrap();
        assert_eq!(fetched_target.id, new_target.id);

        let deleted_target_operation_id = target_service.delete(&new_target.id, None).unwrap();
        assert_eq!(deleted_target_operation_id.id.len(), 36);
    }

    #[test]
    fn test_get_matching_host() {
        dotenv().ok();

        let host = Host {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
        };

        let matching_target: Target = Target {
            id: "eastus2".to_owned(),
            labels: vec!["location:eastus2".to_string()],
        };

        let non_matching_target: Target = Target {
            id: "westus2".to_owned(),
            labels: vec!["location:westus2".to_string()],
        };

        let target_persistence = MemoryPersistence::<Target>::default();

        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let target_service = TargetService {
            persistence: Box::new(target_persistence),
            event_stream,
        };

        target_service.create(&matching_target, &None).unwrap();
        target_service.create(&non_matching_target, &None).unwrap();

        let matching_targets = target_service.get_matching_host(&host).unwrap();
        assert_eq!(matching_targets.len(), 1);
        assert_eq!(matching_targets[0].id, matching_target.id);
    }
}
