use async_trait::async_trait;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::persistence::{Persistable, Persistence};

#[derive(Debug)]
pub struct MemoryPersistence<Model>
where
    Model: Persistable<Model>,
{
    models: Arc<Mutex<HashMap<String, Model>>>,
}

#[async_trait]
impl<Model> Persistence<Model> for MemoryPersistence<Model>
where
    Model: Persistable<Model>,
{
    async fn upsert(&self, model: &Model) -> anyhow::Result<u64> {
        let mut locked_models = self.get_models_locked()?;

        locked_models.insert(model.get_id(), model.clone());

        Ok(1)
    }

    async fn delete(&self, model_id: &str) -> anyhow::Result<u64> {
        let mut locked_models = self.get_models_locked()?;

        locked_models.remove_entry(&model_id.to_string());

        Ok(1)
    }

    async fn list(&self) -> anyhow::Result<Vec<Model>> {
        let locked_models = self.get_models_locked()?;

        let models = locked_models.values().cloned().collect();

        Ok(models)
    }

    async fn get_by_id(&self, model_id: &str) -> anyhow::Result<Option<Model>> {
        let locked_models = self.get_models_locked()?;

        match locked_models.get(model_id) {
            Some(fetched_model) => Ok(Some(fetched_model.clone())),
            None => Ok(None),
        }
    }
}

impl<Model> Default for MemoryPersistence<Model>
where
    Model: Persistable<Model>,
{
    fn default() -> Self {
        Self {
            models: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<Model> MemoryPersistence<Model>
where
    Model: Persistable<Model>,
{
    fn get_models_locked(&self) -> anyhow::Result<MutexGuard<HashMap<String, Model>>> {
        match self.models.lock() {
            Ok(locked_models) => Ok(locked_models),
            Err(_) => Err(anyhow::anyhow!("failed to acquire lock")),
        }
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::get_host_fixture;

    use super::*;

    use crate::models::Host;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();

        let host_persistence = MemoryPersistence::<Host>::default();
        let host: Host = get_host_fixture(None).into();

        host_persistence.upsert(&host).await.unwrap();

        let fetched_host = host_persistence.get_by_id(&host.id).await.unwrap().unwrap();

        assert_eq!(fetched_host.id, host.id);
        assert_eq!(fetched_host.labels.len(), 2);

        let deleted_hosts = host_persistence.delete(&host.id).await.unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
