use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    models::Deployment,
    persistence::{DeploymentPersistence, PersistableModel, Persistence},
};

pub struct DeploymentMemoryPersistence {
    models: Arc<Mutex<HashMap<String, Deployment>>>,
}

impl Persistence<Deployment> for DeploymentMemoryPersistence {
    fn create(&self, deployment: &Deployment) -> anyhow::Result<String> {
        let mut locked_deployments = self.get_models_locked()?;

        locked_deployments.insert(deployment.get_id(), deployment.clone());

        Ok(deployment.get_id())
    }

    fn create_many(&self, deployments: &[Deployment]) -> anyhow::Result<Vec<String>> {
        let mut deployment_ids = Vec::new();
        for (_, deployment) in deployments.iter().enumerate() {
            let deployment_id = self.create(deployment)?;
            deployment_ids.push(deployment_id);
        }

        Ok(deployment_ids)
    }

    fn delete(&self, deployment_id: &str) -> anyhow::Result<usize> {
        let mut locked_deployments = self.get_models_locked()?;

        locked_deployments.remove_entry(&deployment_id.to_string());

        Ok(1)
    }

    fn delete_many(&self, deployment_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, deployment_id) in deployment_ids.iter().enumerate() {
            self.delete(deployment_id)?;
        }

        Ok(deployment_ids.len())
    }

    fn get_by_id(&self, deployment_id: &str) -> anyhow::Result<Option<Deployment>> {
        let locked_deployments = self.get_models_locked()?;

        match locked_deployments.get(deployment_id) {
            Some(fetched_deployment) => Ok(Some(fetched_deployment.clone())),
            None => Ok(None),
        }
    }

    fn list(&self) -> anyhow::Result<Vec<Deployment>> {
        let locked_deployments = self.get_models_locked()?;

        let deployments = locked_deployments.values().cloned().collect();

        Ok(deployments)
    }
}

impl DeploymentPersistence for DeploymentMemoryPersistence {
    fn get_by_target_id(&self, target_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let locked_deployments = self.get_models_locked()?;

        let mut deployments_for_target = Vec::new();
        for deployment in (*locked_deployments).values() {
            if deployment.target_id == target_id {
                deployments_for_target.push(deployment.clone());
            }
        }

        Ok(deployments_for_target)
    }

    fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let locked_deployments = self.get_models_locked()?;

        let mut deployments_for_template = Vec::new();
        for deployment in (*locked_deployments).values() {
            if deployment.template_id == Some(template_id.to_string()) {
                deployments_for_template.push(deployment.clone());
            }
        }

        Ok(deployments_for_template)
    }

    fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let locked_deployments = self.get_models_locked()?;

        let mut deployments_for_workload = Vec::new();
        for deployment in (*locked_deployments).values() {
            if deployment.workload_id == workload_id {
                deployments_for_workload.push(deployment.clone());
            }
        }

        Ok(deployments_for_workload)
    }
}

impl DeploymentMemoryPersistence {
    fn get_models_locked(&self) -> anyhow::Result<MutexGuard<HashMap<String, Deployment>>> {
        match self.models.lock() {
            Ok(locked_deployments) => Ok(locked_deployments),
            Err(_) => Err(anyhow::anyhow!("failed to acquire lock")),
        }
    }
}

impl Default for DeploymentMemoryPersistence {
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

        let new_deployment = Deployment {
            id: "deployment-service-under-test".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            target_id: "target-fixture".to_owned(),
            template_id: None,
            host_count: 3,
        };

        let deployment_persistence = DeploymentMemoryPersistence::default();

        let inserted_deployment_id = deployment_persistence.create(&new_deployment).unwrap();
        assert_eq!(inserted_deployment_id, new_deployment.id);

        let fetched_deployment = deployment_persistence
            .get_by_id(&inserted_deployment_id)
            .unwrap()
            .unwrap();

        assert_eq!(fetched_deployment.id, new_deployment.id);

        let deployments_for_target = deployment_persistence
            .get_by_target_id(&new_deployment.target_id)
            .unwrap();

        assert_eq!(deployments_for_target.len(), 1);

        let deleted_deployments = deployment_persistence
            .delete(&inserted_deployment_id)
            .unwrap();
        assert_eq!(deleted_deployments, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();

        let new_deployment = Deployment {
            id: "deployment-service-many-under-test".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            target_id: "target-fixture".to_owned(),
            template_id: Some("external-service".to_string()),
            host_count: 3,
        };

        let deployment_persistence = DeploymentMemoryPersistence::default();

        let inserted_host_ids = deployment_persistence
            .create_many(&[new_deployment.clone()])
            .unwrap();
        assert_eq!(inserted_host_ids.len(), 1);
        assert_eq!(inserted_host_ids[0], new_deployment.id);

        let deleted_hosts = deployment_persistence
            .delete_many(&[&new_deployment.id])
            .unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
