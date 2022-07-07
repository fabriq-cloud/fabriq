use dotenv::dotenv;
use http::Request;
use hyper::Body;
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
    GrpcWorkspaceService,
};
use akira::persistence::relational::{
    AssignmentRelationalPersistence, ConfigRelationalPersistence, DeploymentRelationalPersistence,
    HostRelationalPersistence, TargetRelationalPersistence, WorkspaceRelationalPersistence,
};
use akira::persistence::relational::{
    TemplateRelationalPersistence, WorkloadRelationalPersistence,
};
use akira::services::{
    AssignmentService, ConfigService, DeploymentService, HostService, TargetService,
    TemplateService, WorkloadService, WorkspaceService,
};
use akira_core::{
    AssignmentServer, DeploymentServer, HealthServer, HostServer, TargetServer, TemplateServer,
    WorkloadServer, WorkspaceServer,
};
use akira_core::{ConfigServer, EventStream};
use akira_mqtt_stream::MqttEventStream;

const DEFAULT_SERVICE_CLIENT_ID: &str = "service";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(DEFAULT_SERVICE_CLIENT_ID)
        .install_simple()
        .expect("Failed to instantiate OpenTelemetry / Jaeger tracing");

    tracing_subscriber::registry() //(1)
        .with(tracing_subscriber::EnvFilter::from_default_env()) //(2)
        .with(tracing_opentelemetry::layer().with_tracer(tracer)) //(3)
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .expect("Failed to register tracer with registry");

    let mqtt_broker_uri = env::var("MQTT_BROKER_URI").expect("MQTT_BROKER_URI must be set");
    let reconciler_client_id =
        env::var("RECONCILER_CLIENT_ID").unwrap_or_else(|_| DEFAULT_SERVICE_CLIENT_ID.to_string());

    let mqtt_event_stream = MqttEventStream::new(&mqtt_broker_uri, &reconciler_client_id, true)?;

    let event_stream: Arc<Box<dyn EventStream>> = Arc::new(Box::new(mqtt_event_stream));

    let assignment_persistence = Box::new(AssignmentRelationalPersistence::default());
    let assignment_service = Arc::new(AssignmentService {
        persistence: assignment_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let config_persistence = Box::new(ConfigRelationalPersistence::default());
    let config_service = Arc::new(ConfigService {
        persistence: config_persistence,
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

    let workspace_grpc_service = WorkspaceServer::with_interceptor(
        GrpcWorkspaceService::new(Arc::clone(&workspace_service)),
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
        .add_service(tonic_web::enable(workspace_grpc_service))
        .add_service(tonic_web::enable(target_grpc_service))
        .add_service(tonic_web::enable(template_grpc_service))
        .serve(addr)
        .await?;

    Ok(())
}
