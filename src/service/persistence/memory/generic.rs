use akira_core::{PersistableModel, Persistence};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

pub struct MemoryPersistence<Model>
where
    Model: PersistableModel<Model> + Clone + Send + Sync,
{
    models: Arc<Mutex<HashMap<String, Model>>>,
}
impl<Model> Persistence<Model> for MemoryPersistence<Model>
where
    Model: PersistableModel<Model> + Clone + Send + Sync,
{
    fn create(&self, new_model: Model) -> anyhow::Result<String> {
        let model = Model::new(new_model);

        let mut locked_models = self.get_models_locked()?;

        locked_models.insert(model.get_id(), model.clone());

        Ok(model.get_id())
    }

    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let mut locked_models = self.get_models_locked()?;

        locked_models.remove_entry(&model_id.to_string());

        Ok(1)
    }

    fn list(&self) -> anyhow::Result<Vec<Model>> {
        let locked_models = self.get_models_locked()?;

        let models = locked_models.values().cloned().collect();

        Ok(models)
    }

    fn get_by_id(&self, model_id: &str) -> anyhow::Result<Option<Model>> {
        let locked_models = self.get_models_locked()?;

        match locked_models.get(model_id) {
            Some(fetched_model) => Ok(Some(fetched_model.clone())),
            None => Ok(None),
        }
    }
}

impl<Model> Default for MemoryPersistence<Model>
where
    Model: PersistableModel<Model> + Clone + Send + Sync,
{
    fn default() -> Self {
        Self {
            models: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<Model> MemoryPersistence<Model>
where
    Model: PersistableModel<Model> + Clone + Send + Sync,
{
    fn get_models_locked(&self) -> anyhow::Result<MutexGuard<HashMap<String, Model>>> {
        match self.models.lock() {
            Ok(locked_assignments) => Ok(locked_assignments),
            Err(_) => Err(anyhow::anyhow!("failed to acquire lock")),
        }
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;

    use crate::models::Host;

    #[test]
    fn test_create_get_delete() {
        dotenv().ok();

        let new_host = Host {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
        };

        let host_persistence = MemoryPersistence::<Host>::default();

        let inserted_host_id = host_persistence.create(new_host.clone()).unwrap();
        assert_eq!(inserted_host_id, new_host.id);

        let fetched_host = host_persistence
            .get_by_id(&inserted_host_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_host.id, new_host.id);
        assert_eq!(fetched_host.labels.len(), 2);

        let deleted_hosts = host_persistence.delete(&inserted_host_id).unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
