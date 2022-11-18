use fabriq_core::{
    create_event, DeploymentMessage, EventStream, EventType, ModelType, OperationId,
};
use std::sync::Arc;

use crate::{models::Deployment, persistence::DeploymentPersistence};

use super::{AssignmentService, ConfigService, TargetService};

#[derive(Debug)]
pub struct DeploymentService {
    pub persistence: Box<dyn DeploymentPersistence>,
    pub event_stream: Arc<dyn EventStream>,

    pub assignment_service: Arc<AssignmentService>,
    pub config_service: Arc<ConfigService>,
    pub target_service: Arc<TargetService>,
}

impl DeploymentService {
    #[tracing::instrument(name = "service::deployment::create")]
    pub async fn upsert(
        &self,
        deployment: &Deployment,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let expected_deployment_id =
            DeploymentMessage::make_id(&deployment.workload_id, &deployment.name);

        if deployment.id != expected_deployment_id {
            let message = format!(
                "deployment id {} doesn't match expected id {}",
                deployment.id, expected_deployment_id
            );

            tracing::error!(message);
            return Err(anyhow::anyhow!(message));
        }

        let target = self.target_service.get_by_id(&deployment.target_id).await?;

        if target.is_none() {
            let message = format!(
                "target id {} not found: can't create deployment {}",
                deployment.target_id, deployment.id
            );

            tracing::error!(message);
            return Err(anyhow::anyhow!(message));
        }

        let affected_count = self.persistence.upsert(deployment).await?;
        let operation_id = OperationId::unwrap_or_create(operation_id);

        if affected_count > 0 {
            let create_event = create_event::<DeploymentMessage>(
                &None,
                &Some(deployment.clone().into()),
                EventType::Created,
                ModelType::Deployment,
                &operation_id,
            );

            self.event_stream.send(&create_event).await?;
        }

        tracing::info!("deployment created: {:?}", deployment);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::deployment::delete")]
    pub async fn delete(
        &self,
        deployment_id: &str,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let deployment = match self.get_by_id(deployment_id).await? {
            Some(deployment) => deployment,
            None => return Err(anyhow::anyhow!("Deployment id {deployment_id} not found")),
        };

        self.assignment_service
            .delete_by_deployment_id(deployment_id, &operation_id)
            .await?;

        self.config_service
            .delete_by_deployment_id(deployment_id, &operation_id)
            .await?;

        let deleted_count = self.persistence.delete(deployment_id).await?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Deployment id {deployment_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(operation_id);

        let delete_event = create_event::<DeploymentMessage>(
            &Some(deployment.clone().into()),
            &None,
            EventType::Deleted,
            ModelType::Deployment,
            &operation_id,
        );

        self.event_stream.send(&delete_event).await?;

        tracing::info!("deployment deleted: {:?}", deployment);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::deployment::get_by_id")]
    pub async fn get_by_id(&self, deployment_id: &str) -> anyhow::Result<Option<Deployment>> {
        self.persistence.get_by_id(deployment_id).await
    }

    #[tracing::instrument(name = "service::deployment::get_by_target_id")]
    pub async fn get_by_target_id(&self, target_id: &str) -> anyhow::Result<Vec<Deployment>> {
        self.persistence.get_by_target_id(target_id).await
    }

    #[tracing::instrument(name = "service::deployment::get_by_template_id")]
    pub async fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Deployment>> {
        self.persistence.get_by_template_id(template_id).await
    }

    #[tracing::instrument(name = "service::deployment::get_by_workload_id")]
    pub async fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Deployment>> {
        self.persistence.get_by_workload_id(workload_id).await
    }

    #[tracing::instrument(name = "service::deployment::list")]
    pub async fn list(&self) -> anyhow::Result<Vec<Deployment>> {
        let results = self.persistence.list().await?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{Assignment, Config, Target},
        persistence::memory::{
            AssignmentMemoryPersistence, ConfigMemoryPersistence, DeploymentMemoryPersistence,
            MemoryPersistence,
        },
    };
    use fabriq_core::test::{
        get_assignment_fixture, get_deployment_fixture, get_keyvalue_config_fixture,
        get_target_fixture,
    };
    use fabriq_memory_stream::MemoryEventStream;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();

        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let target_persistence = MemoryPersistence::<Target>::default();
        let target_service = Arc::new(TargetService {
            persistence: Box::new(target_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let target: Target = get_target_fixture(None).into();
        let operation_id = target_service.upsert(&target, &None).await.unwrap();

        let config_persistence = ConfigMemoryPersistence::default();
        let config_service = Arc::new(ConfigService {
            persistence: Box::new(config_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let config: Config = get_keyvalue_config_fixture().into();
        config_service
            .upsert(&config, &Some(operation_id.clone()))
            .await
            .unwrap();

        let assignment_persistence = AssignmentMemoryPersistence::default();
        let assignment_service = Arc::new(AssignmentService {
            persistence: Box::new(assignment_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let assignment: Assignment = get_assignment_fixture(None).into();
        assignment_service
            .upsert(&assignment, &Some(operation_id.clone()))
            .await
            .unwrap();

        let deployment_persistence = DeploymentMemoryPersistence::default();
        let deployment_service = DeploymentService {
            persistence: Box::new(deployment_persistence),
            event_stream: Arc::clone(&event_stream),

            assignment_service,
            config_service,
            target_service,
        };

        let deployment: Deployment = get_deployment_fixture(None).into();

        let deployment_created_operation_id = deployment_service
            .upsert(&deployment, &Some(operation_id))
            .await
            .unwrap();
        assert_eq!(deployment_created_operation_id.id.len(), 36);

        let fetched_deployment = deployment_service
            .get_by_id(&deployment.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_deployment.id, deployment.id);

        let deployments_by_target = deployment_service
            .get_by_target_id(&deployment.target_id)
            .await
            .unwrap();

        assert!(!deployments_by_target.is_empty());

        let deleted_operation_id = deployment_service
            .delete(&deployment.id, &None)
            .await
            .unwrap();
        assert_eq!(deleted_operation_id.id.len(), 36);
    }
}
