use akira_core::{
    git::{remote::RemoteGitRepoFactory, RemoteGitRepo},
    EventStream,
};
use akira_postgresql_stream::PostgresqlEventStream;
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

const DEFAULT_GITOPS_CONSUMER_ID: &str = "gitops";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(DEFAULT_GITOPS_CONSUMER_ID)
        .install_simple()
        .expect("Failed to instantiate OpenTelemetry / Jaeger tracing");

    tracing_subscriber::registry() //(1)
        .with(tracing_subscriber::EnvFilter::from_default_env()) //(2)
        .with(tracing_opentelemetry::layer().with_tracer(tracer)) //(3)
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .expect("Failed to register tracer with registry");

    let gitops_consumer_id =
        env::var("GITOPS_CONSUMER_ID").unwrap_or_else(|_| DEFAULT_GITOPS_CONSUMER_ID.to_string());

    let event_stream: Arc<Box<dyn EventStream>> = Arc::new(Box::new(PostgresqlEventStream::new()?));

    let repo_url = env::var("GITOPS_REPO_URL").expect("GITOPS_REPO_URL must be set");
    let repo_branch = env::var("GITOPS_REPO_BRANCH").unwrap_or_else(|_| "main".to_owned());
    let private_ssh_key_path =
        env::var("GITOPS_PRIVATE_SSH_KEY_PATH").expect("GITOPS_PRIVATE_SSH_KEY_PATH must be set");
    let private_ssh_key = fs::read_to_string(&private_ssh_key_path)?;

    let gitops_repo = Arc::new(RemoteGitRepo::new(
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

    let template_repo_factory = Arc::new(RemoteGitRepoFactory {});

    let mut gitops_processor = GitOpsProcessor {
        gitops_repo,
        private_ssh_key,

        template_repo_factory,

        config_client,
        deployment_client,
        template_client,
        workload_client,
    };

    tracing::info!("gitops processor: starting event loop");

    for event in event_stream
        .receive(&gitops_consumer_id)
        .into_iter()
        .flatten()
    {
        gitops_processor.process(&event).await?;
        event_stream.delete(&event, &gitops_consumer_id)?;
    }

    Ok(())
}
