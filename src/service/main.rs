use axum::{response::Html, routing::get, Router};
use http::Request;
use hyper::Body;
use opentelemetry::sdk::trace as sdktrace;
use opentelemetry::trace::TraceError;
use opentelemetry_otlp::WithExportConfig;
use sqlx::postgres::PgPoolOptions;
use std::{env, sync::Arc};
use tokio::time::Duration;
use tonic::{codegen::http, transport::Server};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::prelude::*;
use url::Url;

use fabriq_core::{
    AssignmentServer, ConfigServer, DeploymentServer, EventStream, HealthServer, HostServer,
    TargetServer, TemplateServer, WorkloadServer,
};
use fabriq_postgresql_stream::PostgresqlEventStream;

mod acl;
mod api;
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

use api::{
    GrpcAssignmentService, GrpcConfigService, GrpcDeploymentService, GrpcHealthService,
    GrpcHostService, GrpcTargetService, GrpcTemplateService, GrpcWorkloadService,
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

async fn health() -> Html<&'static str> {
    Html("ok")
}

fn init_tracer() -> Result<sdktrace::Tracer, TraceError> {
    let opentelemetry_endpoint =
        env::var("OTEL_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_owned());
    let opentelemetry_endpoint =
        Url::parse(&opentelemetry_endpoint).expect("OTEL_ENDPOINT is not a valid url");

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(opentelemetry_endpoint.as_str()),
        )
        .install_batch(opentelemetry::runtime::Tokio)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let http_router = Router::new().route("/health", get(health));
    let http_services = http_router.into_make_service();

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

    let config_persistence = Box::new(ConfigRelationalPersistence {
        db: Arc::clone(&db),
    });
    let config_service = Arc::new(ConfigService {
        persistence: config_persistence,
        event_stream: Arc::clone(&event_stream),

        deployment_service: Arc::clone(&deployment_service),
        workload_service: Arc::clone(&workload_service),
    });

    let endpoint = env::var("ENDPOINT").unwrap_or_else(|_| "[::1]:50051".to_owned());
    let addr = endpoint.parse()?;

    let assignment_grpc_service = AssignmentServer::with_interceptor(
        GrpcAssignmentService::new(Arc::clone(&assignment_service)),
        acl::authorize,
    );

    let config_grpc_service = ConfigServer::with_interceptor(
        GrpcConfigService::new(Arc::clone(&config_service)),
        acl::authorize,
    );

    let deployment_grpc_service = DeploymentServer::with_interceptor(
        GrpcDeploymentService::new(Arc::clone(&deployment_service)),
        acl::authorize,
    );

    let health_grpc_service =
        HealthServer::with_interceptor(GrpcHealthService::default(), acl::authorize);

    let host_grpc_service = HostServer::with_interceptor(
        GrpcHostService::new(Arc::clone(&host_service)),
        acl::authorize,
    );

    let target_grpc_service = TargetServer::with_interceptor(
        GrpcTargetService::new(Arc::clone(&target_service)),
        acl::authorize,
    );

    let template_grpc_service = TemplateServer::with_interceptor(
        GrpcTemplateService::new(Arc::clone(&template_service)),
        acl::authorize,
    );

    let workload_grpc_service = WorkloadServer::with_interceptor(
        GrpcWorkloadService::new(Arc::clone(&workload_service)),
        acl::authorize,
    );

    tracing::info!("grpc services listening on {}", addr);

    let tracing_layer = ServiceBuilder::new().layer(TraceLayer::new_for_grpc().make_span_with(
        |request: &Request<Body>| {
            tracing::info_span!(
                "gRPC",
                http.method = %request.method(),
                http.url = %request.uri(),
                http.status_code = tracing::field::Empty,
                otel.name = %format!("gRPC {}", request.method()),
                otel.kind = "client",
                otel.status_code = tracing::field::Empty,
            )
        },
    ));

    let grpc_services = Server::builder()
        .layer(tracing_layer)
        .add_service(tonic_web::enable(assignment_grpc_service))
        .add_service(tonic_web::enable(config_grpc_service))
        .add_service(tonic_web::enable(deployment_grpc_service))
        .add_service(tonic_web::enable(health_grpc_service))
        .add_service(tonic_web::enable(host_grpc_service))
        .add_service(tonic_web::enable(workload_grpc_service))
        .add_service(tonic_web::enable(target_grpc_service))
        .add_service(tonic_web::enable(template_grpc_service))
        .into_service();

    let hybrid_services = hybrid_service(http_services, grpc_services);
    let api_future = hyper::Server::bind(&addr).serve(hybrid_services);

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
