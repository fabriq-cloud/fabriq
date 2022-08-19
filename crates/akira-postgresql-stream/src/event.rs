use std::time::SystemTime;

use crate::schema::event_queue;
use akira_core::{Event, OperationId};
use diesel::{Associations, Identifiable, Insertable, Queryable, QueryableByName};

#[derive(
    Associations, Clone, Debug, Eq, Identifiable, Insertable, PartialEq, Queryable, QueryableByName,
)]
#[table_name = "event_queue"]
pub struct EventModel {
    pub id: String,
    pub event_timestamp: SystemTime,
    pub consumer_id: String,

    pub operation_id: Option<String>,
    pub model_type: i32,

    pub serialized_current_model: Option<Vec<u8>>,
    pub serialized_previous_model: Option<Vec<u8>>,

    pub event_type: i32,
}

impl EventModel {
    pub fn make_id(operation_id: &str, consumer_id: &str) -> String {
        format!("{}-{}", operation_id, consumer_id)
    }
}

impl From<EventModel> for Event {
    fn from(model: EventModel) -> Self {
        let operation_id = OperationId {
            id: model.operation_id.unwrap(),
        };

        Self {
            timestamp: Some(model.event_timestamp.into()),
            operation_id: Some(operation_id),
            model_type: model.model_type,
            serialized_current_model: model.serialized_current_model,
            serialized_previous_model: model.serialized_previous_model,
            event_type: model.event_type,
        }
    }
}
