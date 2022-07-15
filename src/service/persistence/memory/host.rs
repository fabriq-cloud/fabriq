use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    models::{Host, Target},
    persistence::{HostPersistence, PersistableModel, Persistence},
};

#[derive(Debug)]
pub struct HostMemoryPersistence {
    models: Arc<Mutex<HashMap<String, Host>>>,
}

impl Persistence<Host> for HostMemoryPersistence {
    fn create(&self, host: &Host) -> anyhow::Result<usize> {
        let mut locked_hosts = self.get_models_locked()?;

        locked_hosts.insert(host.get_id(), host.clone());

        Ok(1)
    }

    fn create_many(&self, hosts: &[Host]) -> anyhow::Result<usize> {
        for (_, host) in hosts.iter().enumerate() {
            self.create(host)?;
        }

        Ok(hosts.len())
    }

    fn delete(&self, host_id: &str) -> anyhow::Result<usize> {
        let mut locked_hosts = self.get_models_locked()?;

        locked_hosts.remove_entry(&host_id.to_string());

        Ok(1)
    }

    fn delete_many(&self, host_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, host_id) in host_ids.iter().enumerate() {
            self.delete(host_id)?;
        }

        Ok(host_ids.len())
    }

    fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Host>> {
        let locked_hosts = self.get_models_locked()?;

        match locked_hosts.get(host_id) {
            Some(fetched_host) => Ok(Some(fetched_host.clone())),
            None => Ok(None),
        }
    }

    fn list(&self) -> anyhow::Result<Vec<Host>> {
        let locked_hosts = self.get_models_locked()?;

        let hosts = locked_hosts.values().cloned().collect();

        Ok(hosts)
    }
}

impl HostPersistence for HostMemoryPersistence {
    fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>> {
        let locked_hosts = self.get_models_locked()?;

        let mut hosts_for_target = Vec::new();
        for host in (*locked_hosts).values() {
            let mut matches = true;
            for label in target.labels.iter() {
                if !host.labels.contains(label) {
                    matches = false;
                }
            }

            if matches {
                hosts_for_target.push(host.clone());
            }
        }

        Ok(hosts_for_target)
    }
}

impl Default for HostMemoryPersistence {
    fn default() -> Self {
        Self {
            models: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl HostMemoryPersistence {
    fn get_models_locked(&self) -> anyhow::Result<MutexGuard<HashMap<String, Host>>> {
        match self.models.lock() {
            Ok(locked_hosts) => Ok(locked_hosts),
            Err(_) => Err(anyhow::anyhow!("failed to acquire lock")),
        }
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::{get_host_fixture, get_target_fixture};

    use super::*;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let host_persistence = HostMemoryPersistence::default();
        let host: Host = get_host_fixture(Some("host-create")).into();

        let created_count = host_persistence.create(&host).unwrap();
        assert_eq!(created_count, 1);

        let fetched_host = host_persistence.get_by_id(&host.id).unwrap().unwrap();

        assert_eq!(fetched_host.id, host.id);

        let target = get_target_fixture(None).into();

        let hosts_for_target = host_persistence.get_matching_target(&target).unwrap();

        assert_eq!(hosts_for_target.len(), 1);

        let deleted_hosts = host_persistence.delete(&host.id).unwrap();
        assert_eq!(deleted_hosts, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();

        let host: Host = get_host_fixture(Some("host-create-many")).into();

        let host_persistence = HostMemoryPersistence::default();

        let created_count = host_persistence.create_many(&[host.clone()]).unwrap();
        assert_eq!(created_count, 1);

        let deleted_hosts = host_persistence.delete_many(&[&host.id]).unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
