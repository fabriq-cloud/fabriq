use std::sync::Arc;
use std::time::SystemTime;

use akira_core::{Event, EventStream, EventType, HostMessage, ModelType, OperationId};
use prost::Message;
use prost_types::Timestamp;

use crate::{
    models::{Host, Target},
    persistence::HostPersistence,
};

pub struct HostService {
    persistence: Box<dyn HostPersistence>,
    event_stream: Arc<Box<dyn EventStream>>,
}

impl HostService {
    pub fn new(
        persistence: Box<dyn HostPersistence>,
        event_stream: Arc<Box<dyn EventStream>>,
    ) -> Self {
        Self {
            persistence,
            event_stream,
        }
    }

    pub async fn create(
        &self,
        host: Host,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let host_id = self.persistence.create(host).await?;

        let host = self.get_by_id(&host_id).await?;
        let host = match host {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Couldn't find created host id returned")),
        };

        let operation_id = OperationId::unwrap_or_create(operation_id);
        let host_message: HostMessage = host.into();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let create_host_event = Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Host as i32,
            serialized_model: host_message.encode_to_vec(),
            event_type: EventType::Created as i32,
            timestamp: Some(timestamp),
        };

        self.event_stream.send(&create_host_event).await?;

        Ok(operation_id)
    }

    pub async fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Host>> {
        self.persistence.get_by_id(host_id).await
    }

    pub async fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>> {
        self.persistence.get_matching_target(target).await
    }

    pub async fn delete(
        &self,
        host_id: &str,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let host = match self.get_by_id(host_id).await? {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Deployment id {host_id} not found")),
        };

        let deleted_count = self.persistence.delete(host_id).await?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Host id {host_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(operation_id);
        let host_message: HostMessage = host.into();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let delete_host_event = Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Host as i32,
            serialized_model: host_message.encode_to_vec(),
            event_type: EventType::Created as i32,
            timestamp: Some(timestamp),
        };

        self.event_stream.send(&delete_host_event).await?;

        Ok(operation_id)
    }

    pub async fn list(&self) -> anyhow::Result<Vec<Host>> {
        let results = self.persistence.list().await?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use akira_memory_stream::MemoryEventStream;
    use dotenv::dotenv;

    use crate::persistence::memory::HostMemoryPersistence;

    use super::*;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        let new_host = Host {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
            //            cpu_capacity: 4000,
            //            memory_capacity: 24000,
        };

        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let host_persistence = HostMemoryPersistence::default();

        let cloned_event_stream = event_stream.clone();
        let host_service = HostService::new(Box::new(host_persistence), cloned_event_stream);

        let created_host_operation_id = host_service
            .create(new_host.clone(), Some(OperationId::create()))
            .await
            .unwrap();
        assert_eq!(created_host_operation_id.id.len(), 36);

        let fetched_host = host_service.get_by_id(&new_host.id).await.unwrap().unwrap();
        assert_eq!(fetched_host.id, new_host.id);

        let deleted_host_operation_id = host_service.delete(&new_host.id, None).await.unwrap();
        assert_eq!(deleted_host_operation_id.id.len(), 36);
    }
}
