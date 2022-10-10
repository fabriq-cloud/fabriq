use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    models::Workload,
    persistence::{Persistable, Persistence, WorkloadPersistence},
};

#[derive(Debug)]
pub struct WorkloadMemoryPersistence {
    models: Arc<Mutex<HashMap<String, Workload>>>,
}

#[async_trait]
impl Persistence<Workload> for WorkloadMemoryPersistence {
    async fn upsert(&self, workload: &Workload) -> anyhow::Result<u64> {
        let mut locked_workloads = self.get_models_locked()?;

        locked_workloads.insert(workload.get_id(), workload.clone());

        Ok(1)
    }

    async fn delete(&self, workload_id: &str) -> anyhow::Result<u64> {
        let mut locked_workloads = self.get_models_locked()?;

        locked_workloads.remove_entry(&workload_id.to_string());

        Ok(1)
    }

    async fn get_by_id(&self, workload_id: &str) -> anyhow::Result<Option<Workload>> {
        let locked_workloads = self.get_models_locked()?;

        match locked_workloads.get(workload_id) {
            Some(fetched_workload) => Ok(Some(fetched_workload.clone())),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<Workload>> {
        let locked_workloads = self.get_models_locked()?;

        let workloads = locked_workloads.values().cloned().collect();

        Ok(workloads)
    }
}

#[async_trait]
impl WorkloadPersistence for WorkloadMemoryPersistence {
    async fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Workload>> {
        let locked_workloads = self.get_models_locked()?;

        let mut workloads_for_template = Vec::new();
        for workload in (*locked_workloads).values() {
            if workload.template_id == template_id {
                workloads_for_template.push(workload.clone());
            }
        }

        Ok(workloads_for_template)
    }
}

impl WorkloadMemoryPersistence {
    fn get_models_locked(&self) -> anyhow::Result<MutexGuard<HashMap<String, Workload>>> {
        match self.models.lock() {
            Ok(locked_workloads) => Ok(locked_workloads),
            Err(_) => Err(anyhow::anyhow!("failed to acquire lock")),
        }
    }
}

impl Default for WorkloadMemoryPersistence {
    fn default() -> Self {
        Self {
            models: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::get_workload_fixture;

    use super::*;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();

        let workload_persistence = WorkloadMemoryPersistence::default();
        let workload = get_workload_fixture(None).into();

        let created_count = workload_persistence.upsert(&workload).await.unwrap();
        assert_eq!(created_count, 1);

        let fetched_workload = workload_persistence
            .get_by_id(&workload.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_workload.id, workload.id);

        let workloads_for_target = workload_persistence
            .get_by_template_id(&workload.template_id)
            .await
            .unwrap();

        assert_eq!(workloads_for_target.len(), 1);

        let deleted_workloads = workload_persistence.delete(&workload.id).await.unwrap();
        assert_eq!(deleted_workloads, 1);
    }
}
