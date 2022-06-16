use akira_core::{Event, EventStream, EventType, ModelType, OperationId, TemplateMessage};
use prost::Message;
use prost_types::Timestamp;
use std::{sync::Arc, time::SystemTime};

use crate::{models::Template, persistence::Persistence};

pub struct TemplateService {
    pub persistence: Box<dyn Persistence<Template>>,
    pub event_stream: Arc<Box<dyn EventStream + 'static>>,
}

impl TemplateService {
    pub fn serialize_model(model: &Option<Template>) -> Option<Vec<u8>> {
        match model {
            Some(assignment) => {
                let message: TemplateMessage = assignment.clone().into();
                Some(message.encode_to_vec())
            }
            None => None,
        }
    }

    pub fn create_event(
        previous_model: &Option<Template>,
        current_model: &Option<Template>,
        event_type: EventType,
        operation_id: &OperationId,
    ) -> Event {
        let serialized_previous_model = Self::serialize_model(previous_model);
        let serialized_current_model = Self::serialize_model(current_model);

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Template as i32,
            serialized_previous_model,
            serialized_current_model,
            event_type: event_type as i32,
            timestamp: Some(timestamp),
        }
    }

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

        let template_id = self.persistence.create(template)?;

        let template = self.get_by_id(&template_id)?;
        let template = match template {
            Some(template) => template,
            None => {
                return Err(anyhow::anyhow!(
                    "Couldn't find created template id returned"
                ))
            }
        };

        let operation_id = OperationId::unwrap_or_create(&operation_id);
        let create_event =
            Self::create_event(&None, &Some(template), EventType::Created, &operation_id);

        self.event_stream.send(&create_event)?;

        Ok(operation_id)
    }

    pub fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Template>> {
        self.persistence.get_by_id(host_id)
    }

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
        let delete_event =
            Self::create_event(&Some(template), &None, EventType::Deleted, &operation_id);

        self.event_stream.send(&delete_event)?;

        Ok(operation_id)
    }

    pub fn list(&self) -> anyhow::Result<Vec<Template>> {
        let results = self.persistence.list()?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use akira_memory_stream::MemoryEventStream;
    use dotenv::dotenv;

    use super::*;
    use crate::persistence::memory::MemoryPersistence;

    #[test]
    fn test_create_get_delete() {
        dotenv().ok();

        let new_template = Template {
            id: "external-service".to_owned(),
            repository: "http://github.com/timfpark/deployment-templates".to_owned(),
            branch: "main".to_owned(),
            path: "external-service".to_owned(),
        };

        let template_persistence = MemoryPersistence::<Template>::default();
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let template_service = TemplateService {
            persistence: Box::new(template_persistence),
            event_stream,
        };

        let create_operation_id = template_service.create(&new_template, None).unwrap();

        assert_eq!(create_operation_id.id.len(), 36);

        let fetched_template = template_service
            .get_by_id(&new_template.id)
            .unwrap()
            .unwrap();

        assert_eq!(fetched_template.id, new_template.id);

        let delete_operation_id = template_service
            .delete(&new_template.id, Some(create_operation_id))
            .unwrap();

        assert_eq!(delete_operation_id.id.len(), 36);
    }
}
