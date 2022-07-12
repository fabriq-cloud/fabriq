use akira::{
    persistence::relational::{
        AssignmentRelationalPersistence, DeploymentRelationalPersistence,
        HostRelationalPersistence, TargetRelationalPersistence, TemplateRelationalPersistence,
        WorkloadRelationalPersistence, WorkspaceRelationalPersistence,
    },
    services::{
        AssignmentService, DeploymentService, HostService, TargetService, TemplateService,
        WorkloadService, WorkspaceService,
    },
};
use akira_core::EventStream;
use akira_mqtt_stream::MqttEventStream;
use dotenv::dotenv;
use std::{env, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod reconciler;

pub use reconciler::Reconciler;

const DEFAULT_RECONCILER_CLIENT_ID: &str = "reconciler";

fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(DEFAULT_RECONCILER_CLIENT_ID)
        .install_simple()
        .expect("Failed to instantiate OpenTelemetry / Jaeger tracing");

    tracing_subscriber::registry() //(1)
        .with(tracing_subscriber::EnvFilter::from_default_env()) //(2)
        .with(tracing_opentelemetry::layer().with_tracer(tracer)) //(3)
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .expect("Failed to register tracer with registry");

    tracing::info!("reconciler: starting");

    let mqtt_broker_uri = env::var("MQTT_BROKER_URI").expect("MQTT_BROKER_URI must be set");
    let gitops_client_id = env::var("RECONCILER_CLIENT_ID")
        .unwrap_or_else(|_| DEFAULT_RECONCILER_CLIENT_ID.to_string());

    let event_stream: Arc<dyn EventStream> = Arc::new(MqttEventStream::new(
        &mqtt_broker_uri,
        &gitops_client_id,
        true,
    )?);

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

    let workspace_persistence = Box::new(WorkspaceRelationalPersistence::default());
    let workspace_service = Arc::new(WorkspaceService {
        persistence: workspace_persistence,
        event_stream: Arc::clone(&event_stream),

        workload_service: Arc::clone(&workload_service),
    });

    let reconciler = Reconciler {
        assignment_service,
        deployment_service,
        host_service,
        target_service,
        template_service,
        workload_service,
        workspace_service,
    };

    tracing::info!("reconciler: starting event loop");

    for event in event_stream.receive().into_iter().flatten() {
        reconciler.process(&event)?;
    }

    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}
