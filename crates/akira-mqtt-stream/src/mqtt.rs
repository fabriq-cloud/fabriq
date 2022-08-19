use std::{fmt, time::Duration};

use akira_core::{Event, EventStream};
use mqtt::{Message as MqttMessage, Receiver};
use paho_mqtt as mqtt;
use prost::Message;

pub struct MqttEventStream {
    client: mqtt::Client,
    rx: Receiver<Option<MqttMessage>>,
}

impl fmt::Debug for MqttEventStream {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("MqttEventStream").finish()
    }
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

    fn send_many(&self, events: &[Event]) -> anyhow::Result<()> {
        for event in events.iter() {
            self.send(event)?;
        }

        Ok(())
    }

    fn receive(&self, _: &str) -> anyhow::Result<Vec<Event>> {
        let messages: Vec<Event> = self
            .rx
            .iter()
            .filter_map(|msg| {
                if let Some(msg) = msg {
                    Event::decode(msg.payload()).ok()
                } else {
                    None
                }
            })
            .collect();

        Ok(messages)
    }

    fn delete(&self, _: &Event, _: &str) -> anyhow::Result<usize> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use std::{env, time::SystemTime};

    use akira_core::{Event, EventType, HostMessage, ModelType, OperationId};
    use prost::Message;
    use prost_types::Timestamp;

    use super::*;

    const DEFAULT_MQTT_BROKER_URI: &str = "tcp://localhost:1883";

    #[test]
    fn test_send_create_host_event() {
        let host = HostMessage {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
        };

        let mqtt_broker_uri =
            env::var("MQTT_BROKER_URI").unwrap_or_else(|_| DEFAULT_MQTT_BROKER_URI.to_owned());

        let host_stream = MqttEventStream::new(&mqtt_broker_uri, "mqtt-stream-test", true).unwrap();
        let operation_id = OperationId::create();

        println!("setup finished");

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

        println!("sending event");

        host_stream.send(&create_host_event).unwrap();

        println!("event sent");

        let received_events = host_stream.receive("").unwrap();

        println!("event received");

        assert_eq!(received_events.len(), 1);

        let received_event = received_events.first().unwrap();

        assert_eq!(received_event.event_type, EventType::Created as i32);
        assert_eq!(received_event.model_type, ModelType::Host as i32);

        let host: HostMessage = HostMessage::decode(
            received_event
                .serialized_current_model
                .as_ref()
                .unwrap()
                .as_slice(),
        )
        .unwrap();

        assert_eq!(host.id, "azure-eastus2-1");
    }
}
