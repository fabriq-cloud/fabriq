use diesel::RunQueryDsl;
use std::time::SystemTime;

use akira_core::{Event, EventStream};

use crate::diesel::ExpressionMethods;
use crate::diesel::QueryDsl;
use crate::event::EventModel;
use crate::schema::event_queue::dsl::event_queue;
use crate::schema::event_queue::{consumer_id, id, table};

#[derive(Debug)]
pub struct PostgresqlEventStream {
    subscribers: Vec<String>,
}

impl PostgresqlEventStream {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            subscribers: vec!["reconciler".to_string(), "gitops".to_string()],
        })
    }
}

impl EventStream for PostgresqlEventStream {
    fn delete(&self, event: &Event, con_id: &str) -> anyhow::Result<usize> {
        if event.operation_id.is_none() {
            return Err(anyhow::anyhow!("operation_id is not supported"));
        }

        let operation_id = event.operation_id.as_ref().unwrap();

        let connection = crate::db::get_connection()?;
        let event_id = EventModel::make_id(&operation_id.id, con_id);

        Ok(diesel::delete(event_queue.filter(id.eq(event_id))).execute(&connection)?)
    }

    fn receive(&self, con_id: &str) -> anyhow::Result<Vec<Event>> {
        let connection = crate::db::get_connection().unwrap();

        let events: Vec<Event> = event_queue
            .filter(consumer_id.eq(con_id))
            .load::<EventModel>(&connection)?
            .into_iter()
            .map(Event::from)
            .collect();

        Ok(events)
    }

    fn send(&self, event: &Event) -> anyhow::Result<()> {
        if event.operation_id.is_none() {
            return Err(anyhow::anyhow!("operation_id is not supported"));
        }

        let operation_id = event.operation_id.as_ref().unwrap();

        let connection = crate::db::get_connection()?;

        let models: Vec<EventModel> = self
            .subscribers
            .iter()
            .map(|con_id| EventModel {
                id: EventModel::make_id(&operation_id.id, con_id),
                event_timestamp: SystemTime::now(),
                consumer_id: con_id.to_string(),

                operation_id: Some(operation_id.id.clone()),

                model_type: event.model_type,
                event_type: event.event_type,

                serialized_current_model: event.serialized_current_model.clone(),
                serialized_previous_model: event.serialized_previous_model.clone(),
            })
            .collect();

        diesel::insert_into(table)
            .values(models)
            .on_conflict(id)
            .do_nothing()
            .execute(&connection)?;

        Ok(())
    }

    fn send_many(&self, events: &[Event]) -> anyhow::Result<()> {
        for event in events {
            self.send(event)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use prost::Message;
    use prost_types::Timestamp;
    use std::time::SystemTime;

    use akira_core::{Event, EventType, HostMessage, ModelType, OperationId};

    use super::*;

    #[test]
    fn test_send_create_host_event() {
        const CONSUMER_ID: &str = "reconciler";

        dotenv().ok();

        let host = HostMessage {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
        };

        let event_stream = PostgresqlEventStream::new().unwrap();
        let operation_id = OperationId::create();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let create_host_event = Event {
            operation_id: Some(operation_id),
            model_type: ModelType::Host as i32,
            serialized_current_model: Some(host.encode_to_vec()),
            serialized_previous_model: None,
            event_type: EventType::Created as i32,
            timestamp: Some(timestamp),
        };

        event_stream.send(&create_host_event).unwrap();

        let received_events = event_stream.receive(CONSUMER_ID).unwrap();

        assert_eq!(received_events.len(), 1);

        let received_event = received_events.first().unwrap();

        assert_eq!(received_event.event_type, EventType::Created as i32);
        assert_eq!(received_event.model_type, ModelType::Host as i32);

        let decoded_host: HostMessage = HostMessage::decode(
            received_event
                .serialized_current_model
                .as_ref()
                .unwrap()
                .as_slice(),
        )
        .unwrap();

        assert_eq!(decoded_host.id, host.id);

        event_stream.delete(received_event, CONSUMER_ID).unwrap();

        let received_events = event_stream.receive(CONSUMER_ID).unwrap();

        assert_eq!(received_events.len(), 0);
    }
}
