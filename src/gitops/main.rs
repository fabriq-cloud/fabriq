use context::Context;
use dotenvy::dotenv;
use fabriq_core::{
    git::{remote::RemoteGitRepoFactory, RemoteGitRepo},
    EventStream,
};
use fabriq_postgresql_stream::PostgresqlEventStream;
use opentelemetry::{
    sdk::{trace as sdktrace, Resource},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use processor::GitOpsProcessor;
use reqwest::Url;
use sqlx::postgres::PgPoolOptions;
use std::{env, sync::Arc};
use tokio::time::Duration;
use tonic::{
    metadata::{Ascii, MetadataValue},
    transport::Channel,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod context;
mod processor;

const DEFAULT_GITOPS_CONSUMER_ID: &str = "gitops";

fn init_tracer() -> anyhow::Result<sdktrace::Tracer> {
    let opentelemetry_endpoint = env::var("OTEL_ENDPOINT").expect("OTEL_ENDPOINT expected");
    let opentelemetry_endpoint =
        Url::parse(&opentelemetry_endpoint).expect("OTEL_ENDPOINT is not a valid url");

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(opentelemetry_endpoint),
        )
        .with_trace_config(
            sdktrace::config().with_resource(Resource::new(vec![KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                "fabriq-gitops",
            )])),
        )
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("init tracer failed");

    Ok(tracer)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let tracer = init_tracer().expect("failed to instantiate opentelemetry tracing");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .expect("Failed to register tracer with registry");

    let gitops_consumer_id =
        env::var("GITOPS_CONSUMER_ID").unwrap_or_else(|_| DEFAULT_GITOPS_CONSUMER_ID.to_string());

    let database_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = Arc::new(
        PgPoolOptions::new()
            .max_connections(20)
            .connect(&database_url)
            .await
            .expect("failed to connect to DATABASE_URL"),
    );

    sqlx::migrate!().run(&*db).await?;

    let subscribers: Vec<String> = dotenvy::var("SUBSCRIBERS")
        .unwrap_or_else(|_| "reconciler,gitops".to_string())
        .split(',')
        .map(|s| s.to_string())
        .collect();

    let event_stream = PostgresqlEventStream {
        db: Arc::clone(&db),
        subscribers,
    };

    let access_token = env::var("GITOPS_ACCESS_TOKEN").expect("GITOPS_ACCESS_TOKEN must be set");
    let access_token: &'static str = Box::leak(Box::new(access_token));

    let api_endpoint = env::var("FABRIQ_API_ENDPOINT").expect("FABRIQ_API_ENDPOINT must be set");
    let api_endpoint: &'static str = Box::leak(Box::new(api_endpoint));

    let repo_url = env::var("GITOPS_REPO_URL").expect("GITOPS_REPO_URL must be set");
    let repo_branch = env::var("GITOPS_REPO_BRANCH").unwrap_or_else(|_| "main".to_owned());

    let private_ssh_key_base64 = env::var("GITOPS_PRIVATE_SSH_KEY_BASE64")
        .expect("GITOPS_PRIVATE_SSH_KEY_BASE64 must be set");
    let private_ssh_key: String = String::from_utf8(base64::decode(&private_ssh_key_base64)?)?;

    let gitops_repo = Arc::new(RemoteGitRepo::new(
        &repo_url,
        &repo_branch,
        &private_ssh_key,
    )?);

    let context = Context::new(api_endpoint, access_token);
    let channel = Channel::from_static(context.endpoint).connect().await?;
    let token: MetadataValue<Ascii> = context.token.parse()?;

    let config_client = Arc::new(fabriq_core::api::client::WrappedConfigClient::new(
        channel.clone(),
        token.clone(),
    ));

    let deployment_client = Arc::new(fabriq_core::api::client::WrappedDeploymentClient::new(
        channel.clone(),
        token.clone(),
    ));

    let template_client = Arc::new(fabriq_core::api::client::WrappedTemplateClient::new(
        channel.clone(),
        token.clone(),
    ));

    let workload_client = Arc::new(fabriq_core::api::client::WrappedWorkloadClient::new(
        channel, token,
    ));

    tracing::info!("starting");

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

    tracing::info!("starting event loop");

    loop {
        tracing::info!("fetching events");

        let events = event_stream.receive(&gitops_consumer_id).await?;

        tracing::info!("event loop fetched {} events", events.len());

        for event in events.iter() {
            tracing::info!(
                "event loop processing event id {} with type {} and model {}",
                event.id,
                event.event_type,
                event.model_type
            );
            match gitops_processor.process(event).await {
                Ok(_) => {
                    tracing::info!("gitops processor: processed event successfully");
                    event_stream.delete(event, &gitops_consumer_id).await?;
                }
                Err(err) => {
                    tracing::error!("gitops processor: failed to process event: {}", err);
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    break;
                }
            };
        }

        if events.is_empty() {
            tracing::info!("event loop sleeping");
            tokio::time::sleep(Duration::from_millis(5000)).await;
        }
    }
}
