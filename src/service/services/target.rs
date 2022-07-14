use akira_core::{create_event, EventStream, EventType, ModelType, OperationId, TargetMessage};
use std::sync::Arc;

use crate::{
    models::{Host, Target},
    persistence::Persistence,
};

#[derive(Debug)]
pub struct TargetService {
    pub persistence: Box<dyn Persistence<Target>>,
    pub event_stream: Arc<dyn EventStream>,
}

impl TargetService {
    #[tracing::instrument(name = "service::target::create")]
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
        let create_event = create_event::<TargetMessage>(
            &None,
            &Some(target.clone().into()),
            EventType::Created,
            ModelType::Target,
            &operation_id,
        );

        self.event_stream.send(&create_event)?;

        tracing::info!("target created: {:?}", target);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::target::get_by_id")]
    pub fn get_by_id(&self, target_id: &str) -> anyhow::Result<Option<Target>> {
        self.persistence.get_by_id(target_id)
    }

    #[tracing::instrument(name = "service::target::delete")]
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
        let delete_event = create_event::<TargetMessage>(
            &Some(target.clone().into()),
            &None,
            EventType::Deleted,
            ModelType::Target,
            &operation_id,
        );

        self.event_stream.send(&delete_event)?;

        tracing::info!("target deleted: {:?}", target);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::target::list")]
    pub fn list(&self) -> anyhow::Result<Vec<Target>> {
        let results = self.persistence.list()?;

        Ok(results)
    }

    #[tracing::instrument(name = "service::target::get_matching_host")]
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
    use super::*;
    use crate::persistence::memory::MemoryPersistence;
    use akira_core::test::{get_host_fixture, get_target_fixture};
    use akira_memory_stream::MemoryEventStream;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let target_persistence = MemoryPersistence::<Target>::default();

        let event_stream =
            Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream + 'static>;

        let target_service = TargetService {
            persistence: Box::new(target_persistence),
            event_stream,
        };

        let target: Target = get_target_fixture(None).into();

        let created_target_operation_id = target_service
            .create(&target, &Some(OperationId::create()))
            .unwrap();
        assert_eq!(created_target_operation_id.id.len(), 36);

        let fetched_target = target_service.get_by_id(&target.id).unwrap().unwrap();
        assert_eq!(fetched_target.id, target.id);

        let deleted_target_operation_id = target_service.delete(&target.id, None).unwrap();
        assert_eq!(deleted_target_operation_id.id.len(), 36);
    }

    #[test]
    fn test_get_matching_host() {
        dotenv::from_filename(".env.test").ok();

        let target_persistence = MemoryPersistence::<Target>::default();
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let target_service = TargetService {
            persistence: Box::new(target_persistence),
            event_stream,
        };

        let host: Host = get_host_fixture(None).into();
        let matching_target = get_target_fixture(None).into();

        let non_matching_target: Target = Target {
            id: "westus2".to_owned(),
            labels: vec!["location:westus2".to_string()],
        };

        target_service.create(&matching_target, &None).unwrap();
        target_service.create(&non_matching_target, &None).unwrap();

        let matching_targets = target_service.get_matching_host(&host).unwrap();
        assert_eq!(matching_targets.len(), 1);
        assert_eq!(matching_targets[0].id, matching_target.id);
    }
}
