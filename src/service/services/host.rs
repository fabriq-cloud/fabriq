use fabriq_core::{create_event, EventStream, EventType, HostMessage, ModelType, OperationId};
use std::sync::Arc;

use crate::{
    models::{Host, Target},
    persistence::HostPersistence,
};

#[derive(Debug)]
pub struct HostService {
    pub persistence: Box<dyn HostPersistence>,
    pub event_stream: Arc<dyn EventStream>,
}

impl HostService {
    #[tracing::instrument(name = "service::host::create")]
    pub async fn upsert(
        &self,
        host: &Host,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let affected_count = self.persistence.upsert(host).await?;

        let operation_id = OperationId::unwrap_or_create(operation_id);

        if affected_count > 0 {
            let create_event = create_event::<HostMessage>(
                &None,
                &Some(host.clone().into()),
                EventType::Created,
                ModelType::Host,
                &operation_id,
            );

            self.event_stream.send(&create_event).await?;
        }

        tracing::info!("host created: {:?}", host);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::host::get_by_id")]
    pub async fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Host>> {
        self.persistence.get_by_id(host_id).await
    }

    #[tracing::instrument(name = "service::host::get_matching_target")]
    pub async fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>> {
        self.persistence.get_matching_target(target).await
    }

    #[tracing::instrument(name = "service::host::delete")]
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

        let operation_id = OperationId::unwrap_or_create(&operation_id);

        let delete_event = create_event::<HostMessage>(
            &Some(host.clone().into()),
            &None,
            EventType::Deleted,
            ModelType::Host,
            &operation_id,
        );

        self.event_stream.send(&delete_event).await?;

        tracing::info!("host deleted: {:?}", host);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::host::list")]
    pub async fn list(&self) -> anyhow::Result<Vec<Host>> {
        let results = self.persistence.list().await?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use fabriq_core::test::get_host_fixture;
    use fabriq_memory_stream::MemoryEventStream;

    use crate::persistence::memory::HostMemoryPersistence;

    use super::*;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();

        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let host_persistence = Box::new(HostMemoryPersistence::default());
        let host: Host = get_host_fixture(None).into();

        let host_service = HostService {
            persistence: host_persistence,
            event_stream,
        };

        let created_host_operation_id = host_service
            .upsert(&host, &Some(OperationId::create()))
            .await
            .unwrap();
        assert_eq!(created_host_operation_id.id.len(), 36);

        let fetched_host = host_service.get_by_id(&host.id).await.unwrap().unwrap();
        assert_eq!(fetched_host.id, host.id);

        let deleted_host_operation_id = host_service.delete(&host.id, None).await.unwrap();
        assert_eq!(deleted_host_operation_id.id.len(), 36);
    }
}
