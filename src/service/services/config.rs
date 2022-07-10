use akira_core::{create_event, ConfigMessage, EventStream, EventType, ModelType, OperationId};
use std::sync::Arc;

use crate::{models::Config, persistence::ConfigPersistence};

#[derive(Debug)]
pub struct ConfigService {
    pub persistence: Box<dyn ConfigPersistence>,
    pub event_stream: Arc<Box<dyn EventStream>>,
}

impl ConfigService {
    #[tracing::instrument(name = "service::config::create")]
    pub fn create(
        &self,
        config: &Config,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let config_id = self.persistence.create(config)?;

        let config = self.get_by_id(&config_id)?;
        let config = match config {
            Some(config) => config,
            None => return Err(anyhow::anyhow!("Couldn't find created config id returned")),
        };

        let operation_id = OperationId::unwrap_or_create(operation_id);
        let create_event = create_event::<ConfigMessage>(
            &None,
            &Some(config.clone().into()),
            EventType::Created,
            ModelType::Config,
            &operation_id,
        );

        self.event_stream.send(&create_event)?;

        tracing::info!("config created: {:?}", config);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::config::get_by_id")]
    pub fn get_by_id(&self, config_id: &str) -> anyhow::Result<Option<Config>> {
        self.persistence.get_by_id(config_id)
    }

    #[tracing::instrument(name = "service::config::delete")]
    pub fn delete(
        &self,
        config_id: &str,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let config = match self.get_by_id(config_id)? {
            Some(config) => config,
            None => return Err(anyhow::anyhow!("Config id {config_id} not found")),
        };

        let deleted_count = self.persistence.delete(config_id)?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Config id {config_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(operation_id);

        let delete_event = create_event::<ConfigMessage>(
            &Some(config.clone().into()),
            &None,
            EventType::Deleted,
            ModelType::Config,
            &operation_id,
        );

        self.event_stream.send(&delete_event)?;

        tracing::info!("config deleted: {:?}", config);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::config::query")]
    pub fn query(&self, deployment_id: &str, workload_id: &str) -> anyhow::Result<Vec<Config>> {
        let mut configs = self.persistence.get_by_workload_id(workload_id)?;
        let mut deployment_config = self.persistence.get_by_deployment_id(deployment_id)?;

        configs.append(&mut deployment_config);

        Ok(configs)
    }

    #[tracing::instrument(name = "service::config::get_by_deployment_id")]
    pub fn get_by_deployment_id(&self, deployment_id: &str) -> anyhow::Result<Vec<Config>> {
        self.persistence.get_by_deployment_id(deployment_id)
    }

    #[tracing::instrument(name = "service::config::get_by_workload_id")]
    pub fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Config>> {
        self.persistence.get_by_workload_id(workload_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::memory::ConfigMemoryPersistence;
    use akira_memory_stream::MemoryEventStream;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let new_config = Config {
            id: "config-persist-single-under-test".to_owned(),

            owning_model: "workload:workload-fixture".to_owned(),

            key: "sample-key".to_owned(),
            value: "sample-value".to_owned(),
        };

        let config_persistence = ConfigMemoryPersistence::default();
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream>);

        let config_service = ConfigService {
            persistence: Box::new(config_persistence),
            event_stream,
        };

        let config_created_operation_id = config_service
            .create(&new_config, &Some(OperationId::create()))
            .unwrap();
        assert_eq!(config_created_operation_id.id.len(), 36);

        let fetched_config = config_service.get_by_id(&new_config.id).unwrap().unwrap();
        assert_eq!(fetched_config.id, new_config.id);

        let configs_by_workload = config_service
            .get_by_workload_id("workload-fixture")
            .unwrap();
        assert_eq!(configs_by_workload.len(), 1);

        let config_for_deployment = config_service
            .query("deployment-fixture", "workload-fixture")
            .unwrap();
        assert_eq!(config_for_deployment.len(), 1);

        let deleted_operation_id = config_service.delete(&new_config.id, &None).unwrap();
        assert_eq!(deleted_operation_id.id.len(), 36);
    }
}
