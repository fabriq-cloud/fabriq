use akira_core::{git::RemoteGitRepo, EventStream};
use akira_mqtt_stream::MqttEventStream;
use context::Context;
use dotenv::dotenv;
use processor::GitOpsProcessor;
use std::{env, fs, sync::Arc};
use tonic::{
    metadata::{Ascii, MetadataValue},
    transport::Channel,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod context;
mod processor;

const DEFAULT_GITOPS_CLIENT_ID: &str = "gitops";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(DEFAULT_GITOPS_CLIENT_ID)
        .install_simple()
        .expect("Failed to instantiate OpenTelemetry / Jaeger tracing");

    tracing_subscriber::registry() //(1)
        .with(tracing_subscriber::EnvFilter::from_default_env()) //(2)
        .with(tracing_opentelemetry::layer().with_tracer(tracer)) //(3)
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .expect("Failed to register tracer with registry");

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
    let repo_branch = env::var("GITOPS_REPO_BRANCH").unwrap_or_else(|_| "main".to_owned());
    let private_ssh_key_path =
        env::var("GITOPS_PRIVATE_SSH_KEY_PATH").expect("GITOPS_PRIVATE_SSH_KEY_PATH must be set");
    let private_ssh_key = fs::read_to_string(&private_ssh_key_path)?;

    let gitops_repo = Arc::new(RemoteGitRepo::new(
        &local_path,
        &repo_url,
        &repo_branch,
        &private_ssh_key,
    )?);

    let context = Context::default();
    let channel = Channel::from_static(context.endpoint).connect().await?;
    let token: MetadataValue<Ascii> = context.token.parse()?;

    let config_client = Arc::new(akira_core::api::client::WrappedConfigClient::new(
        channel.clone(),
        token.clone(),
    ));

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

    tracing::info!("gitops processor: starting");

    let mut gitops_processor = GitOpsProcessor {
        gitops_repo,
        private_ssh_key,

        config_client,
        deployment_client,
        template_client,
        workload_client,
    };

    tracing::info!("gitops processor: ready for events");

    for event in event_stream.receive().into_iter().flatten() {
        tracing::info!("gitops processor: event received: {:?}", event);
        gitops_processor.process(&event).await?;
    }

    Ok(())
}
