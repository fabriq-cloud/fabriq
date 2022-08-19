use akira::{
    persistence::relational::{
        AssignmentRelationalPersistence, DeploymentRelationalPersistence,
        HostRelationalPersistence, TargetRelationalPersistence, TemplateRelationalPersistence,
        WorkloadRelationalPersistence,
    },
    services::{
        AssignmentService, DeploymentService, HostService, TargetService, TemplateService,
        WorkloadService,
    },
};
use akira_core::EventStream;
use akira_postgresql_stream::PostgresqlEventStream;
use dotenv::dotenv;
use std::{env, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod reconciler;

pub use reconciler::Reconciler;

const DEFAULT_RECONCILER_CONSUMER_ID: &str = "reconciler";

fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(DEFAULT_RECONCILER_CONSUMER_ID)
        .install_simple()
        .expect("Failed to instantiate OpenTelemetry / Jaeger tracing");

    tracing_subscriber::registry() //(1)
        .with(tracing_subscriber::EnvFilter::from_default_env()) //(2)
        .with(tracing_opentelemetry::layer().with_tracer(tracer)) //(3)
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .expect("Failed to register tracer with registry");

    tracing::info!("reconciler: starting");

    let reconciler_consumer_id = env::var("RECONCILER_CONSUMER_ID")
        .unwrap_or_else(|_| DEFAULT_RECONCILER_CONSUMER_ID.to_string());

    let event_stream: Arc<dyn EventStream> = Arc::new(PostgresqlEventStream::new()?);

    let assignment_persistence = Box::new(AssignmentRelationalPersistence::default());
    let assignment_service = Arc::new(AssignmentService {
        persistence: assignment_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let deployment_persistence = Box::new(DeploymentRelationalPersistence::default());
    let deployment_service = Arc::new(DeploymentService {
        persistence: deployment_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let host_persistence = Box::new(HostRelationalPersistence::default());
    let host_service = Arc::new(HostService {
        persistence: host_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let target_persistence = Box::new(TargetRelationalPersistence::default());
    let target_service = Arc::new(TargetService {
        persistence: target_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let template_persistence = Box::new(TemplateRelationalPersistence::default());
    let template_service = Arc::new(TemplateService {
        persistence: template_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let workload_persistence = Box::new(WorkloadRelationalPersistence::default());
    let workload_service = Arc::new(WorkloadService {
        persistence: workload_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let reconciler = Reconciler {
        assignment_service,
        deployment_service,
        host_service,
        target_service,
        template_service,
        workload_service,
    };

    tracing::info!("reconciler: starting event loop");

    for event in event_stream
        .receive(&reconciler_consumer_id)
        .into_iter()
        .flatten()
    {
        reconciler.process(&event)?;
        event_stream.delete(&event, &reconciler_consumer_id)?;
    }

    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}
