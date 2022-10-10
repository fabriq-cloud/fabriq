use fabriq_core::{create_event, EventStream, EventType, ModelType, OperationId, WorkloadMessage};
use std::sync::Arc;

use crate::{models::Workload, persistence::WorkloadPersistence};

use super::TemplateService;

#[derive(Debug)]
pub struct WorkloadService {
    pub persistence: Box<dyn WorkloadPersistence>,
    pub event_stream: Arc<dyn EventStream>,

    pub template_service: Arc<TemplateService>,
}

impl WorkloadService {
    #[tracing::instrument(name = "service::workload::create")]
    pub async fn upsert(
        &self,
        workload: &Workload,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let expected_workload_id = WorkloadMessage::make_id(&workload.team_id, &workload.name);

        if workload.id != expected_workload_id {
            return Err(anyhow::anyhow!(
                "workload id {} doesn't match expected id {}",
                workload.id,
                expected_workload_id
            ));
        }

        if workload
            .team_id
            .split(WorkloadMessage::TEAM_ID_SEPARATOR)
            .into_iter()
            .count()
            != 2
        {
            return Err(anyhow::anyhow!(
                "invalid team id, expected format <org>/<team>, found {}",
                workload.team_id
            ));
        }

        let template = self
            .template_service
            .get_by_id(&workload.template_id)
            .await?;

        if template.is_none() {
            return Err(anyhow::anyhow!(
                "template id {} not found: can't create workload {}",
                workload.template_id,
                workload.id
            ));
        }

        let affected_count = self.persistence.upsert(workload).await?;
        let operation_id = OperationId::unwrap_or_create(&operation_id);

        if affected_count > 0 {
            let create_event = create_event::<WorkloadMessage>(
                &None,
                &Some(workload.clone().into()),
                EventType::Created,
                ModelType::Workload,
                &operation_id,
            );

            self.event_stream.send(&create_event).await?;
        }

        tracing::info!("workload created: {:?}", workload);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::workload::get_by_id")]
    pub async fn get_by_id(&self, workload_id: &str) -> anyhow::Result<Option<Workload>> {
        self.persistence.get_by_id(workload_id).await
    }

    #[tracing::instrument(name = "service::workload::delete")]
    pub async fn delete(
        &self,
        workload_id: &str,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let workload = match self.get_by_id(workload_id).await? {
            Some(workload) => workload,
            None => return Err(anyhow::anyhow!("Workload id {workload_id} not found")),
        };

        let deleted_count = self.persistence.delete(workload_id).await?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Workload id {workload_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(&operation_id);
        let delete_event = create_event::<WorkloadMessage>(
            &Some(workload.clone().into()),
            &None,
            EventType::Deleted,
            ModelType::Workload,
            &operation_id,
        );

        self.event_stream.send(&delete_event).await?;

        tracing::info!("workload deleted: {:?}", workload);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::workload::list")]
    pub async fn list(&self) -> anyhow::Result<Vec<Workload>> {
        let results = self.persistence.list().await?;

        Ok(results)
    }

    #[tracing::instrument(name = "service::workload::get_by_template_id")]
    pub async fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Workload>> {
        self.persistence.get_by_template_id(template_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::Template,
        persistence::memory::{MemoryPersistence, WorkloadMemoryPersistence},
    };
    use fabriq_core::test::{get_template_fixture, get_workload_fixture};
    use fabriq_memory_stream::MemoryEventStream;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();

        let workload_persistence = WorkloadMemoryPersistence::default();
        let event_stream: Arc<dyn EventStream> = Arc::new(MemoryEventStream::new().unwrap());

        let template_persistence = MemoryPersistence::<Template>::default();

        let template_service = Arc::new(TemplateService {
            persistence: Box::new(template_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let workload_service = WorkloadService {
            persistence: Box::new(workload_persistence),
            event_stream,

            template_service: Arc::clone(&template_service),
        };

        let template: Template = get_template_fixture(Some("template-fixture")).into();
        let operation_id = template_service.upsert(&template, None).await.unwrap();

        let workload: Workload = get_workload_fixture(None).into();

        let create_operation_id = workload_service
            .upsert(&workload, Some(operation_id))
            .await
            .unwrap();
        assert_eq!(create_operation_id.id.len(), 36);

        let fetched_workload = workload_service
            .get_by_id(&workload.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_workload.id, workload.id);

        let delete_operation_id = workload_service.delete(&workload.id, None).await.unwrap();
        assert_eq!(delete_operation_id.id.len(), 36);
    }
}
