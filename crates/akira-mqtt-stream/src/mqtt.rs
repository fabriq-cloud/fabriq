use std::time::Duration;

use akira_core::{Event, EventStream};
use mqtt::{Message as MqttMessage, Receiver};
use paho_mqtt as mqtt;
use prost::Message;

pub struct MqttEventStream {
    client: mqtt::Client,
    rx: Receiver<Option<MqttMessage>>,
}

const KEEP_ALIVE_INTERVAL: u64 = 20; // seconds
const DEFAULT_CONNECTION_TIMEOUT: u64 = 60; // seconds
const EVENTS_TOPIC: &str = "events";
const MIN_RETRY_INTERVAL: u64 = 1;
const MAX_RETRY_INTERVAL: u64 = 64;

impl MqttEventStream {
    pub fn new(broker_uri: &str, client_id: &str, subscribe: bool) -> anyhow::Result<Self> {
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(broker_uri)
            .client_id(client_id)
            .finalize();

        let mut client = mqtt::Client::new(create_opts)?;

        let rx = client.start_consuming();

        let conn_opts = mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(Duration::from_secs(KEEP_ALIVE_INTERVAL))
            .automatic_reconnect(
                Duration::from_secs(MIN_RETRY_INTERVAL),
                Duration::from_secs(MAX_RETRY_INTERVAL),
            )
            .finalize();

        client.set_timeout(Duration::from_secs(DEFAULT_CONNECTION_TIMEOUT));
        client.connect(conn_opts)?;

        if subscribe {
            let subscriptions = [EVENTS_TOPIC];
            let qos = vec![1; subscriptions.len()];
            client
                .subscribe_many(&subscriptions, &qos)
                .and_then(|rsp| {
                    rsp.subscribe_many_response()
                        .ok_or(mqtt::Error::General("Bad response"))
                })?;
        }

        Ok(MqttEventStream { client, rx })
    }
}

impl EventStream for MqttEventStream {
    fn send(&self, event: &Event) -> anyhow::Result<()> {
        let msg = mqtt::MessageBuilder::new()
            .topic(EVENTS_TOPIC)
            .payload(event.encode_to_vec())
            .qos(1)
            .finalize();

        Ok(self.client.publish(msg)?)
    }

    fn receive(&self) -> Box<dyn Iterator<Item = Option<Event>> + '_> {
        Box::new(self.rx.iter().map(|msg| {
            if let Some(msg) = msg {
                let payload = msg.payload();
                let event = Event::decode(payload).unwrap();
                Some(event)
            } else {
                None
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use akira_core::{Event, EventType, HostMessage, ModelType, OperationId};
    use prost::Message;
    use prost_types::Timestamp;

    use super::*;

    const DEFAULT_BROKER_URI: &str = "tcp://localhost:1883";

    #[test]
    fn test_create_get_delete() {
        let host = HostMessage {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
        };

        let host_stream =
            MqttEventStream::new(DEFAULT_BROKER_URI, "mqtt-stream-test", true).unwrap();
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

        let mut received_iterator = host_stream.receive();
        let received_event = received_iterator.next().unwrap().unwrap();

        assert_eq!(received_event.event_type, EventType::Created as i32);
        assert_eq!(received_event.model_type, ModelType::Host as i32);

        let host: HostMessage =
            HostMessage::decode(received_event.serialized_model.as_slice()).unwrap();

        assert_eq!(host.id, "azure-eastus2-1");
    }
}
