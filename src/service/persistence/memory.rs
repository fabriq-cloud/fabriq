use akira_core::{PersistableModel, Persistence};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

pub struct MemoryPersistence<Model, NewModel>
where
    Model: PersistableModel<Model, NewModel> + Clone + Send + Sync,
    NewModel: Send + Sync,
{
    models: Arc<Mutex<HashMap<String, Model>>>,

    _phantom: std::marker::PhantomData<NewModel>,
}

#[async_trait]
impl<Model, NewModel> Persistence<Model, NewModel> for MemoryPersistence<Model, NewModel>
where
    Model: PersistableModel<Model, NewModel> + Clone + Send + Sync,
    NewModel: Send + Sync,
{
    async fn create(&self, new_model: NewModel) -> anyhow::Result<String> {
        let model = Model::new(new_model);

        let mut locked_hosts = self.models.lock().await;

        locked_hosts.insert(model.get_id(), model.clone());

        Ok(model.get_id())
    }

    async fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let mut locked_hosts = self.models.lock().await;

        locked_hosts.remove_entry(&model_id.to_string());

        Ok(1)
    }

    async fn list(&self) -> anyhow::Result<Vec<Model>> {
        let locked_hosts = self.models.lock().await;

        let mut models = Vec::new();

        for (_, model) in locked_hosts.iter() {
            models.push(model.clone());
        }

        Ok(models)
    }

    async fn get_by_id(&self, model_id: &str) -> anyhow::Result<Option<Model>> {
        let locked_models = self.models.lock().await;

        match locked_models.get(model_id) {
            Some(fetched_model) => Ok(Some(fetched_model.clone())),
            None => Ok(None),
        }
    }
}

impl<Model, NewModel> Default for MemoryPersistence<Model, NewModel>
where
    Model: PersistableModel<Model, NewModel> + Clone + Send + Sync,
    NewModel: Send + Sync,
{
    fn default() -> Self {
        Self {
            models: Arc::new(Mutex::new(HashMap::new())),

            _phantom: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;

    use crate::models::Host;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        let new_host = Host {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
            //            cpu_capacity: 4000,
            //            memory_capacity: 24000,
        };

        let host_persistence = MemoryPersistence::<Host, Host>::default();

        let inserted_host_id = host_persistence.create(new_host.clone()).await.unwrap();
        assert_eq!(inserted_host_id, new_host.id);

        let fetched_host = host_persistence
            .get_by_id(&inserted_host_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_host.id, new_host.id);
        assert_eq!(fetched_host.labels.len(), 2);

        let deleted_hosts = host_persistence.delete(&inserted_host_id).await.unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
