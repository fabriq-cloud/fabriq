use akira_core::{create_event, EventStream, EventType, HostMessage, ModelType, OperationId};
use std::sync::Arc;

use crate::{
    models::{Host, Target},
    persistence::HostPersistence,
};

pub struct HostService {
    pub persistence: Box<dyn HostPersistence>,
    pub event_stream: Arc<Box<dyn EventStream>>,
}

impl HostService {
    pub fn create(
        &self,
        host: &Host,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let host_id = self.persistence.create(host)?;

        let host = self.get_by_id(&host_id)?;
        let host = match host {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Couldn't find created host id returned")),
        };

        let operation_id = OperationId::unwrap_or_create(operation_id);

        let create_event = create_event::<HostMessage>(
            &None,
            &Some(host.into()),
            EventType::Created,
            ModelType::Host,
            &operation_id,
        );

        self.event_stream.send(&create_event)?;

        Ok(operation_id)
    }

    pub fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Host>> {
        self.persistence.get_by_id(host_id)
    }

    pub fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>> {
        self.persistence.get_matching_target(target)
    }

    pub fn delete(
        &self,
        host_id: &str,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let host = match self.get_by_id(host_id)? {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Deployment id {host_id} not found")),
        };

        let deleted_count = self.persistence.delete(host_id)?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Host id {host_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(&operation_id);

        let delete_event = create_event::<HostMessage>(
            &None,
            &Some(host.into()),
            EventType::Deleted,
            ModelType::Host,
            &operation_id,
        );

        self.event_stream.send(&delete_event)?;

        Ok(operation_id)
    }

    pub async fn list(&self) -> anyhow::Result<Vec<Host>> {
        let results = self.persistence.list()?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use akira_memory_stream::MemoryEventStream;
    use dotenv::dotenv;

    use crate::persistence::memory::HostMemoryPersistence;

    use super::*;

    #[test]
    fn test_create_get_delete() {
        dotenv().ok();

        let new_host = Host {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
        };

        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let host_persistence = Box::new(HostMemoryPersistence::default());

        let host_service = HostService {
            persistence: host_persistence,
            event_stream,
        };

        let created_host_operation_id = host_service
            .create(&new_host, &Some(OperationId::create()))
            .unwrap();
        assert_eq!(created_host_operation_id.id.len(), 36);

        let fetched_host = host_service.get_by_id(&new_host.id).unwrap().unwrap();
        assert_eq!(fetched_host.id, new_host.id);

        let deleted_host_operation_id = host_service.delete(&new_host.id, None).unwrap();
        assert_eq!(deleted_host_operation_id.id.len(), 36);
    }
}
