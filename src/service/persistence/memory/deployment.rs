use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    models::Deployment,
    persistence::{DeploymentPersistence, Persistable, Persistence},
};

#[derive(Debug)]
pub struct DeploymentMemoryPersistence {
    models: Arc<Mutex<HashMap<String, Deployment>>>,
}

#[async_trait]
impl Persistence<Deployment> for DeploymentMemoryPersistence {
    async fn upsert(&self, deployment: &Deployment) -> anyhow::Result<u64> {
        let mut locked_deployments = self.get_models_locked()?;

        locked_deployments.insert(deployment.get_id(), deployment.clone());

        Ok(1)
    }

    async fn delete(&self, deployment_id: &str) -> anyhow::Result<u64> {
        let mut locked_deployments = self.get_models_locked()?;

        locked_deployments.remove_entry(&deployment_id.to_string());

        Ok(1)
    }

    async fn get_by_id(&self, deployment_id: &str) -> anyhow::Result<Option<Deployment>> {
        let locked_deployments = self.get_models_locked()?;

        match locked_deployments.get(deployment_id) {
            Some(fetched_deployment) => Ok(Some(fetched_deployment.clone())),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<Deployment>> {
        let locked_deployments = self.get_models_locked()?;

        let deployments = locked_deployments.values().cloned().collect();

        Ok(deployments)
    }
}

#[async_trait]
impl DeploymentPersistence for DeploymentMemoryPersistence {
    async fn get_by_target_id(&self, target_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let locked_deployments = self.get_models_locked()?;

        let mut deployments_for_target = Vec::new();
        for deployment in (*locked_deployments).values() {
            if deployment.target_id == target_id {
                deployments_for_target.push(deployment.clone());
            }
        }

        Ok(deployments_for_target)
    }

    async fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let locked_deployments = self.get_models_locked()?;

        let mut deployments_for_template = Vec::new();
        for deployment in (*locked_deployments).values() {
            if deployment.template_id == Some(template_id.to_string()) {
                deployments_for_template.push(deployment.clone());
            }
        }

        Ok(deployments_for_template)
    }

    async fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Deployment>> {
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
    use fabriq_core::test::get_deployment_fixture;

    use super::*;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();

        let deployment_persistence = DeploymentMemoryPersistence::default();
        let deployment = get_deployment_fixture(None).into();

        let created_count = deployment_persistence.upsert(&deployment).await.unwrap();
        assert_eq!(created_count, 1);

        let fetched_deployment = deployment_persistence
            .get_by_id(&deployment.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_deployment.id, deployment.id);

        let deployments_for_target = deployment_persistence
            .get_by_target_id(&deployment.target_id)
            .await
            .unwrap();

        assert_eq!(deployments_for_target.len(), 1);

        let deleted_deployments = deployment_persistence.delete(&deployment.id).await.unwrap();
        assert_eq!(deleted_deployments, 1);
    }
}
