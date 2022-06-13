use std::{sync::Arc, time::SystemTime};

use akira_core::{
    DeploymentMessage, Event, EventStream, EventType, ModelType, OperationId, Persistence,
};
use prost::Message;
use prost_types::Timestamp;

use crate::models::Deployment;

pub struct DeploymentService {
    pub persistence: Box<dyn Persistence<Deployment>>,
    pub event_stream: Arc<Box<dyn EventStream>>,
}

impl DeploymentService {
    pub fn create(
        &self,
        deployment: Deployment,
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
        let deployment_message: DeploymentMessage = deployment.into();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let create_deployment_event = Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Deployment as i32,
            serialized_model: deployment_message.encode_to_vec(),
            event_type: EventType::Created as i32,
            timestamp: Some(timestamp),
        };

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
        let deployment_message: DeploymentMessage = deployment.into();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let delete_deployment_event = Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Deployment as i32,
            serialized_model: deployment_message.encode_to_vec(),
            event_type: EventType::Deleted as i32,
            timestamp: Some(timestamp),
        };

        self.event_stream.send(&delete_deployment_event)?;

        Ok(operation_id)
    }

    pub fn list(&self) -> anyhow::Result<Vec<Deployment>> {
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

        let new_deployment = Deployment {
            id: "foreign-exchange-api-prod".to_owned(),
            workload_id: "foreign-exchange-api".to_owned(),
            target_id: "azure-east".to_owned(),
            hosts: 3,
        };

        let deployment_persistence = MemoryPersistence::<Deployment>::default();
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let deployment_service = DeploymentService {
            persistence: Box::new(deployment_persistence),
            event_stream,
        };

        let deployment_created_operation_id = deployment_service
            .create(new_deployment.clone(), &Some(OperationId::create()))
            .unwrap();
        assert_eq!(deployment_created_operation_id.id.len(), 36);

        let fetched_deployment = deployment_service
            .get_by_id(&new_deployment.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_deployment.id, new_deployment.id);

        let deleted_operation_id = deployment_service
            .delete(&new_deployment.id, &None)
            .unwrap();
        assert_eq!(deleted_operation_id.id.len(), 36);
    }
}
