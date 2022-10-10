use akira_core::{Event, OperationId};
use sqlx::types::chrono::NaiveDateTime;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostgreSQLEvent {
    pub id: String,
    pub event_timestamp: NaiveDateTime,
    pub consumer_id: String,

    pub operation_id: String,
    pub model_type: i32,

    pub serialized_current_model: Option<Vec<u8>>,
    pub serialized_previous_model: Option<Vec<u8>>,

    pub event_type: i32,
}

impl PostgreSQLEvent {
    pub fn make_id(operation_id: &str, consumer_id: &str) -> String {
        format!("{}-{}", operation_id, consumer_id)
    }
}

impl From<PostgreSQLEvent> for Event {
    fn from(model: PostgreSQLEvent) -> Self {
        let operation_id = OperationId {
            id: model.operation_id,
        };

        let prost_timestamp = prost_types::Timestamp {
            seconds: model.event_timestamp.timestamp(),
            nanos: model.event_timestamp.timestamp_subsec_nanos() as i32,
        };

        Self {
            timestamp: Some(prost_timestamp),
            operation_id: Some(operation_id),
            model_type: model.model_type,
            serialized_current_model: model.serialized_current_model,
            serialized_previous_model: model.serialized_previous_model,
            event_type: model.event_type,
        }
    }
}
