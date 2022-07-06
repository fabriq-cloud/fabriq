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
    fn create(&self, workload: &Workload) -> anyhow::Result<String> {
        let mut locked_workloads = self.get_models_locked()?;

        locked_workloads.insert(workload.get_id(), workload.clone());

        Ok(workload.get_id())
    }

    fn create_many(&self, workloads: &[Workload]) -> anyhow::Result<Vec<String>> {
        let mut workload_ids = Vec::new();
        for (_, workload) in workloads.iter().enumerate() {
            let workload_id = self.create(workload)?;
            workload_ids.push(workload_id);
        }

        Ok(workload_ids)
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
    use super::*;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let new_workload = Workload {
            id: "workload-under-test".to_owned(),
            workspace_id: "workspace-fixture".to_owned(),
            template_id: "template-fixture".to_owned(),
        };

        let workload_persistence = WorkloadMemoryPersistence::default();

        let inserted_workload_id = workload_persistence.create(&new_workload).unwrap();
        assert_eq!(inserted_workload_id, new_workload.id);

        let fetched_workload = workload_persistence
            .get_by_id(&inserted_workload_id)
            .unwrap()
            .unwrap();

        assert_eq!(fetched_workload.id, new_workload.id);

        let workloads_for_target = workload_persistence
            .get_by_template_id(&new_workload.template_id)
            .unwrap();

        assert_eq!(workloads_for_target.len(), 1);

        let deleted_workloads = workload_persistence.delete(&inserted_workload_id).unwrap();
        assert_eq!(deleted_workloads, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();

        let new_workload = Workload {
            id: "workload-under-test".to_owned(),
            workspace_id: "workspace-fixture".to_owned(),
            template_id: "template-fixture".to_owned(),
        };

        let workload_persistence = WorkloadMemoryPersistence::default();

        let inserted_host_ids = workload_persistence
            .create_many(&[new_workload.clone()])
            .unwrap();
        assert_eq!(inserted_host_ids.len(), 1);
        assert_eq!(inserted_host_ids[0], new_workload.id);

        let deleted_hosts = workload_persistence
            .delete_many(&[&new_workload.id])
            .unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
