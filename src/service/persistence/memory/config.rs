use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    models::Config,
    persistence::{ConfigPersistence, Persistable, Persistence},
};

#[derive(Debug)]
pub struct ConfigMemoryPersistence {
    models: Arc<Mutex<HashMap<String, Config>>>,
}

#[async_trait]
impl Persistence<Config> for ConfigMemoryPersistence {
    async fn upsert(&self, config: &Config) -> anyhow::Result<u64> {
        let mut locked_configs = self.get_models_locked()?;

        locked_configs.insert(config.get_id(), config.clone());

        Ok(1)
    }

    async fn delete(&self, config_id: &str) -> anyhow::Result<u64> {
        let mut locked_configs = self.get_models_locked()?;

        locked_configs.remove_entry(&config_id.to_string());

        Ok(1)
    }

    async fn get_by_id(&self, config_id: &str) -> anyhow::Result<Option<Config>> {
        let locked_configs = self.get_models_locked()?;

        match locked_configs.get(config_id) {
            Some(fetched_config) => Ok(Some(fetched_config.clone())),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<Config>> {
        let locked_configs = self.get_models_locked()?;

        let configs = locked_configs.values().cloned().collect();

        Ok(configs)
    }
}

#[async_trait]
impl ConfigPersistence for ConfigMemoryPersistence {
    async fn get_by_deployment_id(&self, deployment_id: &str) -> anyhow::Result<Vec<Config>> {
        let locked_configs = self.get_models_locked()?;

        let mut configs_for_deployment = Vec::new();
        for config in (*locked_configs).values() {
            let (model_type, model_id) = config.split_owning_model();
            if model_type == "deployment" && model_id == deployment_id {
                configs_for_deployment.push(config.clone());
            }
        }

        Ok(configs_for_deployment)
    }

    async fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Config>> {
        let locked_configs = self.get_models_locked()?;

        let mut configs_for_template = Vec::new();
        for config in (*locked_configs).values() {
            let (model_type, model_id) = config.split_owning_model();
            if model_type == "deployment" && model_id == template_id {
                configs_for_template.push(config.clone());
            }
        }

        Ok(configs_for_template)
    }

    async fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Config>> {
        let locked_configs = self.get_models_locked()?;

        let mut configs_for_target = Vec::new();
        for config in (*locked_configs).values() {
            let (model_type, model_id) = config.split_owning_model();
            if model_type == "workload" && model_id == workload_id {
                configs_for_target.push(config.clone());
            }
        }

        Ok(configs_for_target)
    }
}

impl ConfigMemoryPersistence {
    fn get_models_locked(&self) -> anyhow::Result<MutexGuard<HashMap<String, Config>>> {
        match self.models.lock() {
            Ok(locked_configs) => Ok(locked_configs),
            Err(_) => Err(anyhow::anyhow!("failed to acquire lock")),
        }
    }
}

impl Default for ConfigMemoryPersistence {
    fn default() -> Self {
        Self {
            models: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use fabriq_core::test::{get_string_config_fixture, get_workload_fixture};

    use super::*;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();

        let workload = get_workload_fixture(None);

        let config_persistence = ConfigMemoryPersistence::default();
        let config: Config = get_string_config_fixture().into();

        let created_count = config_persistence.upsert(&config).await.unwrap();
        assert_eq!(created_count, 1);

        let fetched_config = config_persistence
            .get_by_id(&config.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_config.id, config.id);

        let configs_for_workload = config_persistence
            .get_by_workload_id(&workload.id)
            .await
            .unwrap();

        assert_eq!(configs_for_workload.len(), 1);

        let deleted_configs = config_persistence.delete(&config.id).await.unwrap();
        assert_eq!(deleted_configs, 1);
    }
}
