use dotenvy::dotenv;
use fabriq::{
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
use fabriq_core::EventStream;
use fabriq_postgresql_stream::PostgresqlEventStream;
use sqlx::postgres::PgPoolOptions;
use std::{env, sync::Arc};
use tokio::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod reconciler;

pub use reconciler::Reconciler;

const DEFAULT_RECONCILER_CONSUMER_ID: &str = "reconciler";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let database_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = Arc::new(
        PgPoolOptions::new()
            .max_connections(20)
            .connect(&database_url)
            .await
            .expect("failed to connect to DATABASE_URL"),
    );

    sqlx::migrate!().run(&*db).await?;

    let reconciler_consumer_id = env::var("RECONCILER_CONSUMER_ID")
        .unwrap_or_else(|_| DEFAULT_RECONCILER_CONSUMER_ID.to_string());

    let event_stream: Arc<dyn EventStream> = Arc::new(PostgresqlEventStream {
        db: Arc::clone(&db),
        subscribers: vec![],
    });

    let assignment_persistence = Box::new(AssignmentRelationalPersistence {
        db: Arc::clone(&db),
    });
    let assignment_service = Arc::new(AssignmentService {
        persistence: assignment_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let target_persistence = Box::new(TargetRelationalPersistence {
        db: Arc::clone(&db),
    });
    let target_service = Arc::new(TargetService {
        persistence: target_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let deployment_persistence = Box::new(DeploymentRelationalPersistence {
        db: Arc::clone(&db),
    });
    let deployment_service = Arc::new(DeploymentService {
        persistence: deployment_persistence,
        event_stream: Arc::clone(&event_stream),

        target_service: Arc::clone(&target_service),
    });

    let host_persistence = Box::new(HostRelationalPersistence {
        db: Arc::clone(&db),
    });
    let host_service = Arc::new(HostService {
        persistence: host_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let template_persistence = Box::new(TemplateRelationalPersistence {
        db: Arc::clone(&db),
    });
    let template_service = Arc::new(TemplateService {
        persistence: template_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let workload_persistence = Box::new(WorkloadRelationalPersistence {
        db: Arc::clone(&db),
    });
    let workload_service = Arc::new(WorkloadService {
        persistence: workload_persistence,
        event_stream: Arc::clone(&event_stream),

        template_service: Arc::clone(&template_service),
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

    loop {
        let events = event_stream.receive(&reconciler_consumer_id).await?;

        for event in events.iter() {
            reconciler.process(event).await?;
            event_stream.delete(event, &reconciler_consumer_id).await?;
        }

        if events.is_empty() {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}
