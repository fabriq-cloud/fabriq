use opentelemetry::{
    sdk::{trace as sdktrace, Resource},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use sqlx::postgres::PgPoolOptions;
use std::{env, sync::Arc};
use tokio::time::Duration;
use tonic::transport::Server;
use tonic_async_interceptor::async_interceptor;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing_subscriber::prelude::*;
use url::Url;

use fabriq_core::{
    AssignmentServer, ConfigServer, DeploymentServer, EventStream, HostServer, TargetServer,
    TemplateServer, WorkloadServer,
};
use fabriq_postgresql_stream::PostgresqlEventStream;

mod acl;
mod grpc;
mod http;
mod hybrid;
mod models;
mod persistence;
mod reconcilation;
mod services;

use hybrid::HybridMakeService;

pub fn hybrid_service<MakeWeb, Grpc>(
    make_web: MakeWeb,
    grpc: Grpc,
) -> HybridMakeService<MakeWeb, Grpc> {
    HybridMakeService { make_web, grpc }
}

use grpc::{
    GrpcAssignmentService, GrpcConfigService, GrpcDeploymentService, GrpcHostService,
    GrpcTargetService, GrpcTemplateService, GrpcWorkloadService,
};

use persistence::relational::{
    AssignmentRelationalPersistence, ConfigRelationalPersistence, DeploymentRelationalPersistence,
    HostRelationalPersistence, TargetRelationalPersistence, TemplateRelationalPersistence,
    WorkloadRelationalPersistence,
};

use reconcilation::Reconciler;

use services::{
    AssignmentService, ConfigService, DeploymentService, HostService, TargetService,
    TemplateService, WorkloadService,
};

const DEFAULT_RECONCILER_CONSUMER_ID: &str = "reconciler";

async fn reconcile(
    reconciler: Reconciler,
    event_stream: Arc<dyn EventStream>,
    consumer_id: &str,
) -> anyhow::Result<()> {
    loop {
        let events = event_stream.receive(consumer_id).await?;

        for event in events.iter() {
            reconciler.process(event).await?;
            event_stream.delete(event, consumer_id).await?;
        }

        if events.is_empty() {
            tokio::time::sleep(Duration::from_millis(5000)).await;
        }
    }
}

fn init_tracer() -> anyhow::Result<sdktrace::Tracer> {
    let opentelemetry_endpoint = env::var("OTEL_ENDPOINT").expect("OTEL_ENDPOINT expected");
    let opentelemetry_endpoint =
        Url::parse(&opentelemetry_endpoint).expect("OTEL_ENDPOINT is not a valid url");

    let service_name = env::var("SERVICE_NAME").unwrap_or_else(|_| "fabriq-api".to_string());
    let service_version = env::var("SERVICE_VERSION").unwrap_or_else(|_| "unknown".to_string());

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(opentelemetry_endpoint),
        )
        .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                service_name,
            ),
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
                service_version,
            ),
        ])))
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("init tracer failed");

    Ok(tracer)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let http_services = http::http_router().into_make_service();

    let tracer = init_tracer().expect("failed to instantiate opentelemetry tracing");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .expect("failed to register tracer with registry");

    let subscribers: Vec<String> = dotenvy::var("SUBSCRIBERS")
        .unwrap_or_else(|_| "reconciler,gitops".to_string())
        .split(',')
        .map(|s| s.to_string())
        .collect();

    let database_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Arc::new(
        PgPoolOptions::new()
            .max_connections(20)
            .connect(&database_url)
            .await
            .expect("failed to connect to DATABASE_URL"),
    );

    sqlx::migrate!().run(&*db).await?;

    let postgresql_event_stream = PostgresqlEventStream {
        db: Arc::clone(&db),
        subscribers,
    };

    let event_stream: Arc<dyn EventStream> = Arc::new(postgresql_event_stream);

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

    let config_persistence = Box::new(ConfigRelationalPersistence {
        db: Arc::clone(&db),
    });
    let config_service = Arc::new(ConfigService {
        persistence: config_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let deployment_persistence = Box::new(DeploymentRelationalPersistence {
        db: Arc::clone(&db),
    });
    let deployment_service = Arc::new(DeploymentService {
        persistence: deployment_persistence,
        event_stream: Arc::clone(&event_stream),

        assignment_service: Arc::clone(&assignment_service),
        config_service: Arc::clone(&config_service),
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

    let endpoint = env::var("ENDPOINT").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let addr = endpoint.parse()?;

    let assignment_grpc_service =
        AssignmentServer::new(GrpcAssignmentService::new(Arc::clone(&assignment_service)));

    let config_grpc_service = ConfigServer::new(GrpcConfigService {
        config_service: Arc::clone(&config_service),
        deployment_service: Arc::clone(&deployment_service),
        workload_service: Arc::clone(&workload_service),
    });

    let deployment_grpc_service =
        DeploymentServer::new(GrpcDeploymentService::new(Arc::clone(&deployment_service)));

    let host_grpc_service = HostServer::new(GrpcHostService::new(Arc::clone(&host_service)));

    let target_grpc_service =
        TargetServer::new(GrpcTargetService::new(Arc::clone(&target_service)));

    let template_grpc_service =
        TemplateServer::new(GrpcTemplateService::new(Arc::clone(&template_service)));

    let workload_grpc_service =
        WorkloadServer::new(GrpcWorkloadService::new(Arc::clone(&workload_service)));

    tracing::info!("grpc services listening on {}", addr);

    let tracing_layer = ServiceBuilder::new().layer(
        TraceLayer::new_for_grpc()
            .make_span_with(tower_http::trace::DefaultMakeSpan::new().level(Level::INFO))
            .on_response(tower_http::trace::DefaultOnResponse::new().level(Level::INFO)),
    );

    let grpc_services = Server::builder()
        .layer(tracing_layer)
        .layer(async_interceptor(acl::authenticate))
        .add_service(assignment_grpc_service)
        .add_service(config_grpc_service)
        .add_service(deployment_grpc_service)
        .add_service(host_grpc_service)
        .add_service(workload_grpc_service)
        .add_service(target_grpc_service)
        .add_service(template_grpc_service)
        .into_service();

    let combined_services = hybrid_service(http_services, grpc_services);
    let api_future = hyper::Server::bind(&addr).serve(combined_services);

    let reconciler = Reconciler {
        assignment_service,
        deployment_service,
        host_service,
        target_service,
        template_service,
        workload_service,
    };

    let reconciler_future = reconcile(reconciler, event_stream, DEFAULT_RECONCILER_CONSUMER_ID);

    tokio::select! {
        r = api_future => {
            tracing::error!("api future failed: {:?}", r);
        },
        r = reconciler_future => {
            tracing::error!("reconciler future failed: {:?}", r);
        }
    };

    Ok(())
}
