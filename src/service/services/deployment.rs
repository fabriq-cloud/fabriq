use akira_core::{create_event, DeploymentMessage, EventStream, EventType, ModelType, OperationId};
use std::sync::Arc;

use crate::{models::Deployment, persistence::DeploymentPersistence};

#[derive(Debug)]
pub struct DeploymentService {
    pub persistence: Box<dyn DeploymentPersistence>,
    pub event_stream: Arc<Box<dyn EventStream>>,
}

impl DeploymentService {
    #[tracing::instrument(name = "service::deployment::create")]
    pub fn create(
        &self,
        deployment: &Deployment,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let deployment_id = self.persistence.create(deployment)?;

        let deployment = self.get_by_id(&deployment_id)?;
        let deployment = match deployment {
            Some(deployment) => deployment,
            None => {
                return Err(anyhow::anyhow!(
                    "Couldn't find created deployment id returned"
                ))
            }
        };

        let operation_id = OperationId::unwrap_or_create(operation_id);
        let create_event = create_event::<DeploymentMessage>(
            &None,
            &Some(deployment.into()),
            EventType::Created,
            ModelType::Deployment,
            &operation_id,
        );

        self.event_stream.send(&create_event)?;

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::deployment::get_by_id")]
    pub fn get_by_id(&self, deployment_id: &str) -> anyhow::Result<Option<Deployment>> {
        self.persistence.get_by_id(deployment_id)
    }

    #[tracing::instrument(name = "service::deployment::delete")]
    pub fn delete(
        &self,
        deployment_id: &str,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let deployment = match self.get_by_id(deployment_id)? {
            Some(deployment) => deployment,
            None => return Err(anyhow::anyhow!("Deployment id {deployment_id} not found")),
        };

        let deleted_count = self.persistence.delete(deployment_id)?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Deployment id {deployment_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(operation_id);

        let delete_event = create_event::<DeploymentMessage>(
            &None,
            &Some(deployment.into()),
            EventType::Deleted,
            ModelType::Deployment,
            &operation_id,
        );

        self.event_stream.send(&delete_event)?;

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::deployment::list")]
    pub fn list(&self) -> anyhow::Result<Vec<Deployment>> {
        let results = self.persistence.list()?;

        Ok(results)
    }

    #[tracing::instrument(name = "service::deployment::get_by_target_id")]
    pub fn get_by_target_id(&self, target_id: &str) -> anyhow::Result<Vec<Deployment>> {
        self.persistence.get_by_target_id(target_id)
    }

    #[tracing::instrument(name = "service::deployment::get_by_template_id")]
    pub fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Deployment>> {
        self.persistence.get_by_template_id(template_id)
    }

    #[tracing::instrument(name = "service::deployment::get_by_workload_id")]
    pub fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Deployment>> {
        self.persistence.get_by_workload_id(workload_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::memory::DeploymentMemoryPersistence;
    use akira_memory_stream::MemoryEventStream;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let new_deployment = Deployment {
            id: "deployment-service-under-test".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            target_id: "target-fixture".to_owned(),
            template_id: Some("external-service".to_string()),
            host_count: 3,
        };

        let deployment_persistence = DeploymentMemoryPersistence::default();
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream>);

        let deployment_service = DeploymentService {
            persistence: Box::new(deployment_persistence),
            event_stream,
        };

        let deployment_created_operation_id = deployment_service
            .create(&new_deployment, &Some(OperationId::create()))
            .unwrap();
        assert_eq!(deployment_created_operation_id.id.len(), 36);

        let fetched_deployment = deployment_service
            .get_by_id(&new_deployment.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_deployment.id, new_deployment.id);

        let deployments_by_target = deployment_service
            .get_by_target_id("target-fixture")
            .unwrap();

        assert!(!deployments_by_target.is_empty());

        let deleted_operation_id = deployment_service
            .delete(&new_deployment.id, &None)
            .unwrap();
        assert_eq!(deleted_operation_id.id.len(), 36);
    }
}
