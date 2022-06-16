use std::{sync::Arc, time::SystemTime};

use akira_core::{DeploymentMessage, Event, EventStream, EventType, ModelType, OperationId};
use prost::Message;
use prost_types::Timestamp;

use crate::{models::Deployment, persistence::DeploymentPersistence};

pub struct DeploymentService {
    pub persistence: Box<dyn DeploymentPersistence>,
    pub event_stream: Arc<Box<dyn EventStream>>,
}

impl DeploymentService {
    pub fn serialize_model(model: &Option<Deployment>) -> Option<Vec<u8>> {
        match model {
            Some(assignment) => {
                let message: DeploymentMessage = assignment.clone().into();
                Some(message.encode_to_vec())
            }
            None => None,
        }
    }

    pub fn create_event(
        previous_deployment: &Option<Deployment>,
        current_deployment: &Option<Deployment>,
        event_type: EventType,
        operation_id: &OperationId,
    ) -> Event {
        let serialized_previous_model = Self::serialize_model(previous_deployment);
        let serialized_current_model = Self::serialize_model(current_deployment);

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Deployment as i32,
            serialized_previous_model,
            serialized_current_model,
            event_type: event_type as i32,
            timestamp: Some(timestamp),
        }
    }

    pub fn create(
        &self,
        deployment: &Deployment,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let deployment_id = self.persistence.create(deployment)?;

        let deployment = self.get_by_id(&deployment_id)?;
        let deployment = match deployment {
            Some(deployment) => deployment,
            None => {
                return Err(anyhow::anyhow!(
                    "Couldn't find created deployment id returned"
                ))
            }
        };

        let operation_id = OperationId::unwrap_or_create(operation_id);

        let create_deployment_event =
            Self::create_event(&None, &Some(deployment), EventType::Created, &operation_id);

        self.event_stream.send(&create_deployment_event)?;

        Ok(operation_id)
    }

    pub fn get_by_id(&self, deployment_id: &str) -> anyhow::Result<Option<Deployment>> {
        self.persistence.get_by_id(deployment_id)
    }

    pub fn delete(
        &self,
        deployment_id: &str,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let deployment = match self.get_by_id(deployment_id)? {
            Some(deployment) => deployment,
            None => return Err(anyhow::anyhow!("Deployment id {deployment_id} not found")),
        };

        let deleted_count = self.persistence.delete(deployment_id)?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Deployment id {deployment_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(operation_id);

        let delete_deployment_event =
            Self::create_event(&Some(deployment), &None, EventType::Deleted, &operation_id);

        self.event_stream.send(&delete_deployment_event)?;

        Ok(operation_id)
    }

    pub fn list(&self) -> anyhow::Result<Vec<Deployment>> {
        let results = self.persistence.list()?;

        Ok(results)
    }

    pub fn get_by_target_id(&self, target_id: &str) -> anyhow::Result<Vec<Deployment>> {
        self.persistence.get_by_target_id(target_id)
    }
}

#[cfg(test)]
mod tests {
    use akira_memory_stream::MemoryEventStream;
    use dotenv::dotenv;

    use super::*;
    use crate::persistence::memory::DeploymentMemoryPersistence;

    #[test]
    fn test_create_get_delete() {
        dotenv().ok();

        let new_deployment = Deployment {
            id: "deployment-service-under-test".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            target_id: "target-fixture".to_owned(),
            template_id: Some("external-service".to_string()),
            host_count: 3,
        };

        let deployment_persistence = DeploymentMemoryPersistence::default();
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream>);

        let deployment_service = DeploymentService {
            persistence: Box::new(deployment_persistence),
            event_stream,
        };

        let deployment_created_operation_id = deployment_service
            .create(&new_deployment, &Some(OperationId::create()))
            .unwrap();
        assert_eq!(deployment_created_operation_id.id.len(), 36);

        let fetched_deployment = deployment_service
            .get_by_id(&new_deployment.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_deployment.id, new_deployment.id);

        let deployments_by_target = deployment_service
            .get_by_target_id("target-fixture")
            .unwrap();

        assert!(!deployments_by_target.is_empty());

        let deleted_operation_id = deployment_service
            .delete(&new_deployment.id, &None)
            .unwrap();
        assert_eq!(deleted_operation_id.id.len(), 36);
    }
}
