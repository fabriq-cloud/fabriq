use std::{env, sync::Arc};

mod context;
mod processor;
mod repo;

use akira_core::EventStream;
use akira_mqtt_stream::MqttEventStream;
use processor::GitOpsProcessor;

const DEFAULT_GITOPS_CLIENT_ID: &str = "reconciler";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mqtt_broker_uri = env::var("MQTT_BROKER_URI").expect("MQTT_BROKER_URI must be set");
    let gitops_client_id =
        env::var("GITOPS_CLIENT_ID").unwrap_or_else(|_| DEFAULT_GITOPS_CLIENT_ID.to_string());

    let event_stream: Arc<Box<dyn EventStream>> = Arc::new(Box::new(MqttEventStream::new(
        &mqtt_broker_uri,
        &gitops_client_id,
        true,
    )?));

    let local_path = env::var("GITOPS_LOCAL_PATH").expect("GITOPS_LOCAL_PATH must be set");
    let repo_url = env::var("GITOPS_REPO_URL").expect("GITOPS_REPO_URL must be set");
    let private_ssh_key_path =
        env::var("GITOPS_PRIVATE_SSH_KEY_PATH").expect("GITOPS_PRIVATE_SSH_KEY_PATH must be set");

    let gitops_repo = repo::GitRepo::new(&local_path, &repo_url, &private_ssh_key_path);

    let gitops_processor = Box::new(GitOpsProcessor::new(gitops_repo).await?);

    gitops_processor.start().await?;
    for event in event_stream.receive().into_iter().flatten() {
        gitops_processor.process(&event).await?;
    }

    Ok(())
}
