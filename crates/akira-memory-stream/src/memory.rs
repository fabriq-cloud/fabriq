use std::sync::Arc;
use std::{collections::VecDeque, sync::Mutex};

use akira_core::{Event, EventStream};

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
    fn send(&self, event: &Event) -> anyhow::Result<()> {
        let mut events = self.events.lock().unwrap();

        events.push_back(event.clone());

        Ok(())
    }

    fn len(&self) -> anyhow::Result<usize> {
        let events = self.events.lock().unwrap();

        Ok(events.len())
    }

    fn is_empty(&self) -> anyhow::Result<bool> {
        let events = self.events.lock().unwrap();

        Ok(events.is_empty())
    }

    fn receive(&self) -> anyhow::Result<Option<Event>> {
        let mut events = self.events.lock().unwrap();

        Ok(events.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use dotenv::dotenv;
    use prost::Message;

    use akira_core::{Event, EventType, HostMessage, ModelType, OperationId};
    use prost_types::Timestamp;

    use super::*;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        let host = HostMessage {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
        };

        let host_stream = MemoryEventStream::new().unwrap();
        let operation_id = OperationId::unwrap_or_create(&None);

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
            serialized_model: host.encode_to_vec(),
            event_type: EventType::Created as i32,
            timestamp: Some(timestamp),
        };

        host_stream.send(&create_host_event).unwrap();

        let events = host_stream.len().unwrap();
        assert_eq!(events, 1);

        let received_event = host_stream.receive().unwrap().unwrap();

        assert_eq!(received_event.event_type, EventType::Created as i32);
        assert_eq!(received_event.model_type, ModelType::Host as i32);

        let host: HostMessage =
            HostMessage::decode(received_event.serialized_model.as_slice()).unwrap();

        assert_eq!(host.id, "azure-eastus2-1");
    }
}
