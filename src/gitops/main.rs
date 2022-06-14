use std::{env, sync::Arc};

mod context;
mod processor;

use akira_core::EventStream;
use akira_mqtt_stream::MqttEventStream;
use processor::GitOpsProcessor;

const DEFAULT_GITOPS_CLIENT_ID: &str = "reconciler";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mqtt_broker_uri = env::var("MQTT_BROKER_URI").expect("MQTT_BROKER_URI must be set");
    let gitops_client_id =
        env::var("GITOPS_CLIENT_ID").unwrap_or_else(|_| DEFAULT_GITOPS_CLIENT_ID.to_string());

    let _event_stream: Arc<Box<dyn EventStream>> = Arc::new(Box::new(MqttEventStream::new(
        &mqtt_broker_uri,
        &gitops_client_id,
        true,
    )?));

    let event_stream: Arc<Box<dyn EventStream>> = Arc::new(Box::new(MqttEventStream::new(
        &mqtt_broker_uri,
        &gitops_client_id,
        true,
    )?));

    let gitops_processor = Box::new(GitOpsProcessor::new().await?);

    for event in event_stream.receive().into_iter().flatten() {
        gitops_processor.process(&event).await?;
    }

    Ok(())
}
