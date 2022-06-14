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

    fn send_many(&self, events: &[Event]) -> anyhow::Result<()> {
        for event in events.iter() {
            self.send(event)?;
        }

        Ok(())
    }

    fn receive(&self) -> Box<dyn Iterator<Item = Option<Event>> + '_> {
        let mut events = self.events.lock().unwrap();

        Box::new(events.drain(..).map(Some).collect::<Vec<_>>().into_iter())
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
    fn test_create_get_delete() {
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
            serialized_model: host.encode_to_vec(),
            event_type: EventType::Created as i32,
            timestamp: Some(timestamp),
        };

        host_stream.send(&create_host_event).unwrap();

        let received_event = host_stream.receive().next().unwrap().unwrap();

        assert_eq!(received_event.event_type, EventType::Created as i32);
        assert_eq!(received_event.model_type, ModelType::Host as i32);

        let host: HostMessage =
            HostMessage::decode(received_event.serialized_model.as_slice()).unwrap();

        assert_eq!(host.id, "azure-eastus2-1");
    }
}
