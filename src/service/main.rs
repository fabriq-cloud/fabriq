use http::Request;
use hyper::Body;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;
use tonic::codegen::http;
use tonic::transport::Server;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use akira::acl;
use akira::api::{
    GrpcAssignmentService, GrpcConfigService, GrpcDeploymentService, GrpcHealthService,
    GrpcHostService, GrpcTargetService, GrpcTemplateService, GrpcWorkloadService,
};
use akira::persistence::relational::{
    AssignmentRelationalPersistence, ConfigRelationalPersistence, DeploymentRelationalPersistence,
    HostRelationalPersistence, TargetRelationalPersistence,
};
use akira::persistence::relational::{
    TemplateRelationalPersistence, WorkloadRelationalPersistence,
};
use akira::services::{
    AssignmentService, ConfigService, DeploymentService, HostService, TargetService,
    TemplateService, WorkloadService,
};
use akira_core::{
    AssignmentServer, DeploymentServer, HealthServer, HostServer, TargetServer, TemplateServer,
    WorkloadServer,
};
use akira_core::{ConfigServer, EventStream};
use akira_postgresql_stream::PostgresqlEventStream;

const DEFAULT_SERVICE_CONSUMER_ID: &str = "service";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(DEFAULT_SERVICE_CONSUMER_ID)
        .install_simple()
        .expect("failed to instantiate opentelemetry tracing");

    tracing_subscriber::registry() //(1)
        .with(tracing_subscriber::EnvFilter::from_default_env()) //(2)
        .with(tracing_opentelemetry::layer().with_tracer(tracer)) //(3)
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

    Server::builder()
        .layer(tracing_layer)
        .add_service(tonic_web::enable(assignment_grpc_service))
        .add_service(tonic_web::enable(config_grpc_service))
        .add_service(tonic_web::enable(deployment_grpc_service))
        .add_service(tonic_web::enable(health_grpc_service))
        .add_service(tonic_web::enable(host_grpc_service))
        .add_service(tonic_web::enable(workload_grpc_service))
        .add_service(tonic_web::enable(target_grpc_service))
        .add_service(tonic_web::enable(template_grpc_service))
        .serve(addr)
        .await?;

    Ok(())
}
