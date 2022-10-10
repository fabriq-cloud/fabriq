use std::sync::Arc;

use crate::{models::Assignment, persistence::AssignmentPersistence};
use fabriq_core::{
    create_event, AssignmentMessage, EventStream, EventType, ModelType, OperationId,
};
use std::fmt::Debug;

#[derive(Debug)]
pub struct AssignmentService {
    pub persistence: Box<dyn AssignmentPersistence>,
    pub event_stream: Arc<dyn EventStream>,
}

impl AssignmentService {
    #[tracing::instrument(name = "service::assignment::create")]
    pub async fn upsert(
        &self,
        assignment: &Assignment,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let operation_id = OperationId::unwrap_or_create(operation_id);

        let affected_count = self.persistence.upsert(assignment).await?;

        if affected_count > 0 {
            let create_event = create_event::<AssignmentMessage>(
                &None,
                &Some(assignment.clone().into()),
                EventType::Created,
                ModelType::Assignment,
                &operation_id,
            );

            self.event_stream.send(&create_event).await?;
        }

        tracing::info!("assignment created: {:?}", assignment);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::assignment::create_many")]
    pub async fn upsert_many(
        &self,
        assignments: &[Assignment],
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let operation_id = OperationId::unwrap_or_create(operation_id);

        for assignment in assignments {
            self.upsert(assignment, &Some(operation_id.clone())).await?;
        }

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::assignment::delete")]
    pub async fn delete(
        &self,
        assignment_id: &str,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let assignment = match self.get_by_id(assignment_id).await? {
            Some(assignment) => assignment,
            None => return Err(anyhow::anyhow!("Deployment id {assignment_id} not found")),
        };

        let deleted_count = self.persistence.delete(assignment_id).await?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Assignment id {assignment_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(operation_id);

        let delete_event = create_event::<AssignmentMessage>(
            &Some(assignment.clone().into()),
            &None,
            EventType::Deleted,
            ModelType::Assignment,
            &operation_id,
        );

        self.event_stream.send(&delete_event).await?;

        tracing::info!("assignment deleted: {:?}", assignment);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::assignment::delete_many")]
    pub async fn delete_many(
        &self,
        assignments: &[Assignment],
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let operation_id = OperationId::unwrap_or_create(operation_id);

        for assignment in assignments {
            self.delete(&assignment.id, &Some(operation_id.clone()))
                .await?;
        }

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::assignment::get_by_id")]
    pub async fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Assignment>> {
        self.persistence.get_by_id(host_id).await
    }

    #[tracing::instrument(name = "service::assignment::get_by_deployment_id")]
    pub async fn get_by_deployment_id(
        &self,
        deployment_id: &str,
    ) -> anyhow::Result<Vec<Assignment>> {
        self.persistence.get_by_deployment_id(deployment_id).await
    }

    #[tracing::instrument(name = "service::assignment::list")]
    pub async fn list(&self) -> anyhow::Result<Vec<Assignment>> {
        let results = self.persistence.list().await?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::memory::AssignmentMemoryPersistence;
    use fabriq_core::test::get_assignment_fixture;
    use fabriq_memory_stream::MemoryEventStream;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();

        let assignment_persistence = AssignmentMemoryPersistence::default();
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;
        let assignment: Assignment =
            get_assignment_fixture(Some("service-assignment-create")).into();

        let assignment_service = AssignmentService {
            persistence: Box::new(assignment_persistence),
            event_stream,
        };

        let create_operation_id = assignment_service.upsert(&assignment, &None).await.unwrap();

        assert_eq!(create_operation_id.id.len(), 36);

        let fetched_assignment = assignment_service
            .get_by_id(&assignment.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_assignment.id, assignment.id);

        let delete_operation_id = assignment_service
            .delete(&assignment.id, &Some(create_operation_id))
            .await
            .unwrap();

        assert_eq!(delete_operation_id.id.len(), 36);
    }

    #[tokio::test]
    async fn test_create_get_delete_many() {
        dotenvy::from_filename(".env.test").ok();

        let assignment_persistence = AssignmentMemoryPersistence::default();
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;
        let assignment: Assignment = get_assignment_fixture(None).into();

        let assignment_service = AssignmentService {
            persistence: Box::new(assignment_persistence),
            event_stream: Arc::clone(&event_stream),
        };

        assignment_service
            .upsert_many(&[assignment.clone()], &None)
            .await
            .unwrap();

        let event_count = event_stream.receive("reconciler").await.unwrap().len();
        assert!(event_count > 0);

        assignment_service
            .delete_many(&[assignment], &None)
            .await
            .unwrap();

        let event_count = event_stream.receive("reconciler").await.unwrap().len();
        assert!(event_count > 0);
    }
}
