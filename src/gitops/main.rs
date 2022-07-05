use std::{env, fs, sync::Arc};

mod context;
mod processor;

use akira_core::{git::RemoteGitRepo, EventStream};
use akira_mqtt_stream::MqttEventStream;
use context::Context;
use processor::GitOpsProcessor;
use tonic::{
    metadata::{Ascii, MetadataValue},
    transport::Channel,
};

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
    let repo_branch = env::var("GITOPS_REPO_BRANCH").expect("GITOPS_REPO_BRANCH must be set");
    let private_ssh_key_path =
        env::var("GITOPS_PRIVATE_SSH_KEY_PATH").expect("GITOPS_PRIVATE_SSH_KEY_PATH must be set");
    let private_ssh_key = fs::read_to_string(&private_ssh_key_path)?;

    let gitops_repo = Arc::new(RemoteGitRepo::new(
        &local_path,
        &repo_url,
        &repo_branch,
        &private_ssh_key_path,
    )?);

    let context = Context::default();
    let channel = Channel::from_static(context.endpoint).connect().await?;
    let token: MetadataValue<Ascii> = context.token.parse()?;

    let deployment_client = Arc::new(akira_core::api::client::WrappedDeploymentClient::new(
        channel.clone(),
        token.clone(),
    ));

    let template_client = Arc::new(akira_core::api::client::WrappedTemplateClient::new(
        channel.clone(),
        token.clone(),
    ));

    let workload_client = Arc::new(akira_core::api::client::WrappedWorkloadClient::new(
        channel, token,
    ));

    let mut gitops_processor = GitOpsProcessor {
        gitops_repo,
        private_ssh_key,

        deployment_client,
        template_client,
        workload_client,
    };

    for event in event_stream.receive().into_iter().flatten() {
        gitops_processor.process(&event).await?;
    }

    Ok(())
}
