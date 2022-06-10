use std::{sync::Arc, time::SystemTime};

use crate::models::Assignment;
use akira_core::{
    AssignmentMessage, Event, EventStream, EventType, ModelType, OperationId, Persistence,
};
use prost::Message;
use prost_types::Timestamp;

pub struct AssignmentService {
    persistence: Box<dyn Persistence<Assignment, Assignment>>,
    event_stream: Arc<Box<dyn EventStream + 'static>>,
}

impl AssignmentService {
    pub fn new(
        persistence: Box<dyn Persistence<Assignment, Assignment>>,
        event_stream: Arc<Box<dyn EventStream>>,
    ) -> Self {
        Self {
            persistence,
            event_stream,
        }
    }

    pub async fn create(
        &self,
        assignment: Assignment,
        operation_id: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        // TODO: Use an Error enumeration to return specific error

        match self.get_by_id(&assignment.id).await? {
            Some(assignment) => {
                return Err(anyhow::anyhow!(
                    "Assignment id {} already exists",
                    assignment.id
                ))
            }
            None => {}
        };

        let assignment_id = self.persistence.create(assignment).await?;

        let assignment = self.get_by_id(&assignment_id).await?;
        let assignment = match assignment {
            Some(assignment) => assignment,
            None => {
                return Err(anyhow::anyhow!(
                    "Couldn't find created assignment id returned"
                ))
            }
        };

        let operation_id = OperationId::unwrap_or_create(operation_id);

        let assignment_message: AssignmentMessage = assignment.into();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let create_assignment_event = Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Assignment as i32,
            serialized_model: assignment_message.encode_to_vec(),
            event_type: EventType::Created as i32,
            timestamp: Some(timestamp),
        };

        self.event_stream.send(&create_assignment_event).await?;

        Ok(operation_id)
    }

    pub async fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Assignment>> {
        self.persistence.get_by_id(host_id).await
    }

    pub async fn delete(
        &self,
        assignment_id: &str,
        operation_id: Option<OperationId>,
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

        let assignment_message: AssignmentMessage = assignment.into();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let delete_assignment_event = Event {
            operation_id: Some(operation_id.clone()),
            model_type: ModelType::Assignment as i32,
            serialized_model: assignment_message.encode_to_vec(),
            event_type: EventType::Deleted as i32,
            timestamp: Some(timestamp),
        };

        self.event_stream.send(&delete_assignment_event).await?;

        Ok(operation_id)
    }

    pub async fn list(&self) -> anyhow::Result<Vec<Assignment>> {
        let results = self.persistence.list().await?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;
    use crate::persistence::memory::MemoryPersistence;
    use akira_memory_stream::MemoryEventStream;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        let new_assignment = Assignment {
            id: "external-service".to_owned(),
            host_id: "host-fixture".to_owned(),
            deployment_id: "deployment-fixture".to_owned(),
        };

        let assignment_persistence = MemoryPersistence::<Assignment, Assignment>::default();
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let assignment_service = AssignmentService {
            persistence: Box::new(assignment_persistence),
            event_stream,
        };

        let create_operation_id = assignment_service
            .create(new_assignment.clone(), None)
            .await
            .unwrap();

        assert_eq!(create_operation_id.id.len(), 36);

        let fetched_assignment = assignment_service
            .get_by_id(&new_assignment.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_assignment.id, new_assignment.id);

        let delete_operation_id = assignment_service
            .delete(&new_assignment.id, Some(create_operation_id))
            .await
            .unwrap();

        assert_eq!(delete_operation_id.id.len(), 36);
    }
}