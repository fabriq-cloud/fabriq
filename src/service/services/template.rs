use akira_core::{create_event, EventStream, EventType, ModelType, OperationId, TemplateMessage};
use std::sync::Arc;

use crate::{models::Template, persistence::Persistence};

#[derive(Debug)]
pub struct TemplateService {
    pub persistence: Box<dyn Persistence<Template>>,
    pub event_stream: Arc<dyn EventStream>,
}

impl TemplateService {
    #[tracing::instrument(name = "service::template::create")]
    pub fn create(
        &self,
        template: &Template,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        // TODO: Use an Error enumeration to return specific error

        match self.get_by_id(&template.id)? {
            Some(template) => {
                return Err(anyhow::anyhow!(
                    "Template id {} already exists",
                    template.id
                ))
            }
            None => {}
        };

        self.persistence.create(template)?;

        let template = self.get_by_id(&template.id)?;
        let template = match template {
            Some(template) => template,
            None => {
                return Err(anyhow::anyhow!(
                    "Couldn't find created template id returned"
                ))
            }
        };

        let operation_id = OperationId::unwrap_or_create(&operation_id);
        let create_event = create_event::<TemplateMessage>(
            &None,
            &Some(template.clone().into()),
            EventType::Created,
            ModelType::Template,
            &operation_id,
        );

        self.event_stream.send(&create_event)?;

        tracing::info!("template created: {:?}", template);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::template::get_by_id")]
    pub fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Template>> {
        self.persistence.get_by_id(host_id)
    }

    #[tracing::instrument(name = "service::template::delete")]
    pub fn delete(
        &self,
        template_id: &str,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let template = match self.get_by_id(template_id)? {
            Some(template) => template,
            None => return Err(anyhow::anyhow!("Template id {template_id} not found")),
        };

        let deleted_count = self.persistence.delete(template_id)?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Template id {template_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(&operation_id);
        let delete_event = create_event::<TemplateMessage>(
            &Some(template.clone().into()),
            &None,
            EventType::Deleted,
            ModelType::Template,
            &operation_id,
        );

        self.event_stream.send(&delete_event)?;

        tracing::info!("template deleted: {:?}", template);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::template::list")]
    pub fn list(&self) -> anyhow::Result<Vec<Template>> {
        let results = self.persistence.list()?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::get_template_fixture;
    use akira_memory_stream::MemoryEventStream;

    use super::*;
    use crate::persistence::memory::MemoryPersistence;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let template_persistence = MemoryPersistence::<Template>::default();
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let template_service = TemplateService {
            persistence: Box::new(template_persistence),
            event_stream,
        };

        let template: Template = get_template_fixture(None).into();

        let create_operation_id = template_service.create(&template, None).unwrap();

        assert_eq!(create_operation_id.id.len(), 36);

        let fetched_template = template_service.get_by_id(&template.id).unwrap().unwrap();

        assert_eq!(fetched_template.id, template.id);

        let delete_operation_id = template_service
            .delete(&template.id, Some(create_operation_id))
            .unwrap();

        assert_eq!(delete_operation_id.id.len(), 36);
    }
}
