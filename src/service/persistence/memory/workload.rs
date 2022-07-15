use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    models::Workload,
    persistence::{PersistableModel, Persistence, WorkloadPersistence},
};

#[derive(Debug)]
pub struct WorkloadMemoryPersistence {
    models: Arc<Mutex<HashMap<String, Workload>>>,
}

impl Persistence<Workload> for WorkloadMemoryPersistence {
    fn create(&self, workload: &Workload) -> anyhow::Result<usize> {
        let mut locked_workloads = self.get_models_locked()?;

        locked_workloads.insert(workload.get_id(), workload.clone());

        Ok(1)
    }

    fn create_many(&self, workloads: &[Workload]) -> anyhow::Result<usize> {
        let mut workload_ids = Vec::new();
        for (_, workload) in workloads.iter().enumerate() {
            let workload_id = self.create(workload)?;
            workload_ids.push(workload_id);
        }

        Ok(workloads.len())
    }

    fn delete(&self, workload_id: &str) -> anyhow::Result<usize> {
        let mut locked_workloads = self.get_models_locked()?;

        locked_workloads.remove_entry(&workload_id.to_string());

        Ok(1)
    }

    fn delete_many(&self, workload_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, workload_id) in workload_ids.iter().enumerate() {
            self.delete(workload_id)?;
        }

        Ok(workload_ids.len())
    }

    fn get_by_id(&self, workload_id: &str) -> anyhow::Result<Option<Workload>> {
        let locked_workloads = self.get_models_locked()?;

        match locked_workloads.get(workload_id) {
            Some(fetched_workload) => Ok(Some(fetched_workload.clone())),
            None => Ok(None),
        }
    }

    fn list(&self) -> anyhow::Result<Vec<Workload>> {
        let locked_workloads = self.get_models_locked()?;

        let workloads = locked_workloads.values().cloned().collect();

        Ok(workloads)
    }
}

impl WorkloadPersistence for WorkloadMemoryPersistence {
    fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Workload>> {
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

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let workload_persistence = WorkloadMemoryPersistence::default();
        let workload = get_workload_fixture(None).into();

        let created_count = workload_persistence.create(&workload).unwrap();
        assert_eq!(created_count, 1);

        let fetched_workload = workload_persistence
            .get_by_id(&workload.id)
            .unwrap()
            .unwrap();

        assert_eq!(fetched_workload.id, workload.id);

        let workloads_for_target = workload_persistence
            .get_by_template_id(&workload.template_id)
            .unwrap();

        assert_eq!(workloads_for_target.len(), 1);

        let deleted_workloads = workload_persistence.delete(&workload.id).unwrap();
        assert_eq!(deleted_workloads, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();

        let workload: Workload = get_workload_fixture(None).into();
        let workload_persistence = WorkloadMemoryPersistence::default();

        let created_count = workload_persistence
            .create_many(&[workload.clone()])
            .unwrap();
        assert_eq!(created_count, 1);

        let deleted_hosts = workload_persistence.delete_many(&[&workload.id]).unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
