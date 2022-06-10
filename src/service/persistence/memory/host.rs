use akira_core::PersistableModel;
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    models::{Host, Target},
    persistence::HostPersistence,
};

pub struct HostMemoryPersistence {
    hosts: Arc<Mutex<HashMap<String, Host>>>,
}

#[async_trait]
impl HostPersistence for HostMemoryPersistence {
    async fn create(&self, host: Host) -> anyhow::Result<String> {
        let mut locked_hosts = self.hosts.lock().await;

        locked_hosts.insert(host.get_id(), host.clone());

        Ok(host.get_id())
    }

    async fn delete(&self, host_id: &str) -> anyhow::Result<usize> {
        let mut locked_hosts = self.hosts.lock().await;

        locked_hosts.remove_entry(&host_id.to_string());

        Ok(1)
    }

    async fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>> {
        let locked_hosts = self.hosts.lock().await;

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

    async fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Host>> {
        let locked_hosts = self.hosts.lock().await;

        match locked_hosts.get(host_id) {
            Some(fetched_host) => Ok(Some(fetched_host.clone())),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<Host>> {
        let locked_hosts = self.hosts.lock().await;

        let mut hosts = Vec::new();

        for (_, host) in locked_hosts.iter() {
            hosts.push(host.clone());
        }

        Ok(hosts)
    }
}

impl Default for HostMemoryPersistence {
    fn default() -> Self {
        Self {
            hosts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        let new_host = Host {
            id: "host-under-test".to_owned(),
            labels: vec!["cloud:azure".to_owned(), "region:eastus2".to_owned()],
        };

        let host_persistence = HostMemoryPersistence::default();

        let inserted_host_id = host_persistence.create(new_host.clone()).await.unwrap();
        assert_eq!(inserted_host_id, new_host.id);

        let fetched_host = host_persistence
            .get_by_id(&inserted_host_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_host.id, new_host.id);

        let target = Target {
            id: "azure".to_owned(),
            labels: vec!["cloud:azure".to_owned()],
        };

        let hosts_for_target = host_persistence.get_matching_target(&target).await.unwrap();

        assert_eq!(hosts_for_target.len(), 1);

        let deleted_hosts = host_persistence.delete(&inserted_host_id).await.unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
