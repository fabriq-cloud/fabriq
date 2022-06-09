use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use tokio::sync::Mutex;

use akira_core::{Event, EventStream};

pub struct MemoryEventStream {
    next_operation_id: AtomicI32,
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl MemoryEventStream {
    pub fn new() -> anyhow::Result<Self> {
        let event_stream = MemoryEventStream {
            next_operation_id: AtomicI32::new(1),
            events: Arc::new(Mutex::new(VecDeque::new())),
        };

        Ok(event_stream)
    }
}

#[async_trait]
impl EventStream for MemoryEventStream {
    async fn fill_operation_id(&self, current_operation_id: Option<i32>) -> anyhow::Result<i32> {
        match current_operation_id {
            Some(current_operation_id) => Ok(current_operation_id),
            None => Ok(self
                .next_operation_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)),
        }
    }

    async fn send(&self, event: &Event) -> anyhow::Result<()> {
        let mut events = self.events.lock().await;

        events.push_back(event.clone());

        Ok(())
    }

    async fn len(&self) -> anyhow::Result<usize> {
        let events = self.events.lock().await;

        Ok(events.len())
    }

    async fn is_empty(&self) -> anyhow::Result<bool> {
        let events = self.events.lock().await;

        Ok(events.is_empty())
    }

    async fn receive(&self) -> anyhow::Result<Option<Event>> {
        let mut events = self.events.lock().await;

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

            cpu_capacity: 4000,
            memory_capacity: 24000,
        };

        let host_stream = MemoryEventStream::new().unwrap();
        let operation_id = OperationId::unwrap_or_create(None);

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

        host_stream.send(&create_host_event).await.unwrap();

        let events = host_stream.len().await.unwrap();
        assert_eq!(events, 1);

        let received_event = host_stream.receive().await.unwrap().unwrap();

        assert_eq!(received_event.event_type, EventType::Created as i32);
        assert_eq!(received_event.model_type, ModelType::Host as i32);

        let host: HostMessage =
            HostMessage::decode(received_event.serialized_model.as_slice()).unwrap();

        assert_eq!(host.id, "azure-eastus2-1");
    }
}
