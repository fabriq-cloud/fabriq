use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    models::Config,
    persistence::{ConfigPersistence, PersistableModel, Persistence},
};

#[derive(Debug)]
pub struct ConfigMemoryPersistence {
    models: Arc<Mutex<HashMap<String, Config>>>,
}

impl Persistence<Config> for ConfigMemoryPersistence {
    fn create(&self, config: &Config) -> anyhow::Result<String> {
        let mut locked_configs = self.get_models_locked()?;

        locked_configs.insert(config.get_id(), config.clone());

        Ok(config.get_id())
    }

    fn create_many(&self, configs: &[Config]) -> anyhow::Result<Vec<String>> {
        let mut config_ids = Vec::new();
        for (_, config) in configs.iter().enumerate() {
            let config_id = self.create(config)?;
            config_ids.push(config_id);
        }

        Ok(config_ids)
    }

    fn delete(&self, config_id: &str) -> anyhow::Result<usize> {
        let mut locked_configs = self.get_models_locked()?;

        locked_configs.remove_entry(&config_id.to_string());

        Ok(1)
    }

    fn delete_many(&self, config_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, config_id) in config_ids.iter().enumerate() {
            self.delete(config_id)?;
        }

        Ok(config_ids.len())
    }

    fn get_by_id(&self, config_id: &str) -> anyhow::Result<Option<Config>> {
        let locked_configs = self.get_models_locked()?;

        match locked_configs.get(config_id) {
            Some(fetched_config) => Ok(Some(fetched_config.clone())),
            None => Ok(None),
        }
    }

    fn list(&self) -> anyhow::Result<Vec<Config>> {
        let locked_configs = self.get_models_locked()?;

        let configs = locked_configs.values().cloned().collect();

        Ok(configs)
    }
}

impl ConfigPersistence for ConfigMemoryPersistence {
    fn get_by_deployment_id(&self, deployment_id: &str) -> anyhow::Result<Vec<Config>> {
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

    fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Config>> {
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

    fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Config>> {
        let locked_configs = self.get_models_locked()?;

        println!("{:?}", locked_configs);

        let mut configs_for_target = Vec::new();
        for config in (*locked_configs).values() {
            let (model_type, model_id) = config.split_owning_model();
            println!("{} {}", model_type, model_id);
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
    use akira_core::test::{
        get_deployment_fixture, get_keyvalue_config_fixture, get_string_config_fixture,
        get_workload_fixture,
    };

    use super::*;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let config: Config = get_string_config_fixture().into();
        let workload = get_workload_fixture(None);

        let config_persistence = ConfigMemoryPersistence::default();

        let inserted_config_id = config_persistence.create(&config).unwrap();
        assert_eq!(inserted_config_id, config.id);

        let fetched_config = config_persistence
            .get_by_id(&inserted_config_id)
            .unwrap()
            .unwrap();

        assert_eq!(fetched_config.id, config.id);

        let configs_for_workload = config_persistence.get_by_workload_id(&workload.id).unwrap();

        assert_eq!(configs_for_workload.len(), 1);

        let deleted_configs = config_persistence.delete(&inserted_config_id).unwrap();
        assert_eq!(deleted_configs, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();

        let config: Config = get_keyvalue_config_fixture().into();
        let deployment = get_deployment_fixture(None);

        let config_persistence = ConfigMemoryPersistence::default();

        let inserted_host_ids = config_persistence.create_many(&[config.clone()]).unwrap();
        assert_eq!(inserted_host_ids.len(), 1);
        assert_eq!(inserted_host_ids[0], config.id);

        let configs_for_deployment = config_persistence
            .get_by_deployment_id(&deployment.id)
            .unwrap();

        assert_eq!(configs_for_deployment.len(), 1);

        let deleted_hosts = config_persistence.delete_many(&[&config.id]).unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
