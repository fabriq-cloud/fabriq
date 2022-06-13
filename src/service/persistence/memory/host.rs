use akira_core::{PersistableModel, Persistence};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    models::{Host, Target},
    persistence::HostPersistence,
};

pub struct HostMemoryPersistence {
    models: Arc<Mutex<HashMap<String, Host>>>,
}

impl Persistence<Host> for HostMemoryPersistence {
    fn create(&self, host: Host) -> anyhow::Result<String> {
        let mut locked_hosts = self.get_models_locked()?;

        locked_hosts.insert(host.get_id(), host.clone());

        Ok(host.get_id())
    }

    fn delete(&self, host_id: &str) -> anyhow::Result<usize> {
        let mut locked_hosts = self.get_models_locked()?;

        locked_hosts.remove_entry(&host_id.to_string());

        Ok(1)
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
            Ok(locked_assignments) => Ok(locked_assignments),
            Err(_) => Err(anyhow::anyhow!("failed to acquire lock")),
        }
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;

    #[test]
    fn test_create_get_delete() {
        dotenv().ok();

        let new_host = Host {
            id: "host-under-test".to_owned(),
            labels: vec!["cloud:azure".to_owned(), "region:eastus2".to_owned()],
        };

        let host_persistence = HostMemoryPersistence::default();

        let inserted_host_id = host_persistence.create(new_host.clone()).unwrap();
        assert_eq!(inserted_host_id, new_host.id);

        let fetched_host = host_persistence
            .get_by_id(&inserted_host_id)
            .unwrap()
            .unwrap();

        assert_eq!(fetched_host.id, new_host.id);

        let target = Target {
            id: "azure".to_owned(),
            labels: vec!["cloud:azure".to_owned()],
        };

        let hosts_for_target = host_persistence.get_matching_target(&target).unwrap();

        assert_eq!(hosts_for_target.len(), 1);

        let deleted_hosts = host_persistence.delete(&inserted_host_id).unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}