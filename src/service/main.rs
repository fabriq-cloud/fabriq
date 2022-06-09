use akira_core::EventStream;
use akira_core::{
    AssignmentServer, DeploymentServer, HealthServer, HostServer, TargetServer, TemplateServer,
    WorkloadServer, WorkspaceServer,
};
use akira_memory_stream::MemoryEventStream;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use tonic::transport::Server;

use akira::acl;
use akira::api::{GrpcAssignmentService, GrpcDeploymentService, GrpcHostService, GrpcHealthService, GrpcTargetService, GrpcTemplateService, GrpcWorkloadService, GrpcWorkspaceService};
use akira::persistence::relational::{
    AssignmentRelationalPersistence, DeploymentRelationalPersistence, HostRelationalPersistence,
    TargetRelationalPersistence, WorkspaceRelationalPersistence,
};
use akira::persistence::relational::{
    TemplateRelationalPersistence, WorkloadRelationalPersistence,
};

use akira::services::{
    AssignmentService, DeploymentService, HostService, TargetService, TemplateService,
    WorkloadService, WorkspaceService,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let event_stream: Arc<Box<dyn EventStream>> = Arc::new(Box::new(MemoryEventStream::new()?));

    let assignment_persistence = Box::new(AssignmentRelationalPersistence::default());
    let assignment_service = Arc::new(AssignmentService::new(
        assignment_persistence,
        Arc::clone(&event_stream),
    ));

    let deployment_persistence = Box::new(DeploymentRelationalPersistence::default());
    let deployment_service = Arc::new(DeploymentService::new(
        deployment_persistence,
        Arc::clone(&event_stream),
    ));

    let host_persistence = Box::new(HostRelationalPersistence::default());
    let host_service = Arc::new(HostService::new(
        host_persistence,
        Arc::clone(&event_stream),
    ));

    let target_persistence = Box::new(TargetRelationalPersistence::default());
    let target_service = Arc::new(TargetService::new(
        target_persistence,
        Arc::clone(&event_stream),
    ));

    let template_persistence = Box::new(TemplateRelationalPersistence::default());
    let template_service = Arc::new(TemplateService::new(
        template_persistence,
        Arc::clone(&event_stream),
    ));

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

    let deployment_grpc_service = DeploymentServer::with_interceptor(
        GrpcDeploymentService::new(Arc::clone(&deployment_service)),
        acl::authorize,
    );

    let health_grpc_service =
        HealthServer::with_interceptor(GrpcHealthService::new(), acl::authorize);

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

    Server::builder()
        .add_service(tonic_web::enable(assignment_grpc_service))
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
