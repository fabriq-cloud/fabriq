use std::sync::Arc;
use std::{collections::VecDeque, sync::Mutex};

use akira_core::{Event, EventStream};

#[derive(Debug)]
pub struct MemoryEventStream {
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl MemoryEventStream {
    pub fn new() -> anyhow::Result<Self> {
        let event_stream = MemoryEventStream {
            events: Arc::new(Mutex::new(VecDeque::new())),
        };

        Ok(event_stream)
    }
}

impl EventStream for MemoryEventStream {
    fn delete(&self, event: &Event, _: &str) -> anyhow::Result<usize> {
        let mut events = self.events.lock().unwrap();

        let starting_len = events.len();
        events.retain(|e| e.operation_id != event.operation_id);
        let deleted_count = starting_len - events.len();

        Ok(deleted_count)
    }

    fn send(&self, event: &Event) -> anyhow::Result<()> {
        let mut events = self.events.lock().unwrap();

        events.push_back(event.clone());

        Ok(())
    }

    fn send_many(&self, events: &[Event]) -> anyhow::Result<()> {
        for event in events.iter() {
            self.send(event)?;
        }

        Ok(())
    }

    fn receive(&self, _: &str) -> anyhow::Result<Vec<Event>> {
        let events = self.events.lock().unwrap();

        Ok(events.clone().iter().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use prost::Message;

    use akira_core::{Event, EventType, HostMessage, ModelType, OperationId};
    use prost_types::Timestamp;

    use super::*;

    #[test]
    fn test_send_create_host_event() {
        let host = HostMessage {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
        };

        let host_stream = MemoryEventStream::new().unwrap();
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

        host_stream.send(&create_host_event).unwrap();

        let received_events = host_stream.receive("").unwrap();

        assert_eq!(received_events.len(), 1);

        let received_event = received_events.first().unwrap();
        assert_eq!(received_event.event_type, EventType::Created as i32);
        assert_eq!(received_event.model_type, ModelType::Host as i32);

        let encoded_message = received_event
            .serialized_current_model
            .as_ref()
            .unwrap()
            .as_slice();

        let host: HostMessage = HostMessage::decode(encoded_message).unwrap();

        assert_eq!(host.id, "azure-eastus2-1");
    }
}
