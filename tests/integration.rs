use std::sync::Arc;

use akira::{
    models::{Deployment, Host, Target, Template, Workload, Workspace},
    persistence::memory::{
        DeploymentMemoryPersistence, HostMemoryPersistence, MemoryPersistence,
        WorkloadMemoryPersistence,
    },
    services::{
        DeploymentService, HostService, TargetService, TemplateService, WorkloadService,
        WorkspaceService,
    },
};
use akira_core::{EventStream, OperationId};
use akira_memory_stream::MemoryEventStream;

#[tokio::test]
async fn test_e2e() {
    let event_stream =
        Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

    let host_persistence = HostMemoryPersistence::default();

    let host_service = HostService {
        persistence: Box::new(host_persistence),
        event_stream: Arc::clone(&event_stream),
    };

    // create a host
    let new_host = Host {
        id: "azure-eastus2-1".to_owned(),
        labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
    };

    host_service
        .create(&new_host, &Some(OperationId::create()))
        .unwrap();

    let target_persistence = MemoryPersistence::<Target>::default();

    let target_service = TargetService {
        persistence: Box::new(target_persistence),
        event_stream: Arc::clone(&event_stream),
    };

    // create target that matches host
    let new_target = Target {
        id: "eastus2".to_string(),
        labels: vec!["location:eastus2".to_string()],
    };

    let create_target_operation_id = target_service.create(&new_target, &None).unwrap();

    assert_eq!(create_target_operation_id.id.len(), 36);

    let template_persistence = MemoryPersistence::<Template>::default();

    let template_service = TemplateService {
        persistence: Box::new(template_persistence),
        event_stream: Arc::clone(&event_stream),
    };

    // create template
    let new_template = Template {
        id: "external-service".to_owned(),
        repository: "http://github.com/timfpark/deployment-templates".to_owned(),
        branch: "main".to_owned(),
        path: "external-service".to_owned(),
    };

    let create_template_operation_id = template_service
        .create(&new_template, Some(OperationId::create()))
        .unwrap();

    assert_eq!(create_template_operation_id.id.len(), 36);

    let workload_persistence = Box::new(WorkloadMemoryPersistence::default());
    let workload_service = Arc::new(WorkloadService {
        persistence: workload_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let workspace_persistence = Box::new(MemoryPersistence::<Workspace>::default());
    let workspace_service = WorkspaceService {
        persistence: workspace_persistence,
        event_stream: Arc::clone(&event_stream),

        workload_service: Arc::clone(&workload_service),
    };

    // create workspace
    let new_workspace = Workspace {
        id: "foreign-exchange".to_string(),
    };

    let create_workspace_operation_id = workspace_service.create(&new_workspace, &None).unwrap();

    assert_eq!(create_workspace_operation_id.id.len(), 36);

    // create workload
    let new_workload = Workload {
        id: "cribbage-api".to_string(),
        workspace_id: new_workspace.id,
        template_id: new_template.id,
    };

    let create_workload_operation_id = workload_service
        .create(&new_workload, Some(OperationId::create()))
        .unwrap();

    assert_eq!(create_workload_operation_id.id.len(), 36);

    let deployment_persistence = DeploymentMemoryPersistence::default();

    let deployment_service = DeploymentService {
        persistence: Box::new(deployment_persistence),
        event_stream: Arc::clone(&event_stream),
    };

    // create deployment
    let new_deployment = Deployment {
        id: "cribbage-api-prod".to_string(),
        workload_id: new_workload.id,
        target_id: new_target.id,
        template_id: Some("external-service".to_string()),
        host_count: 3,
    };

    let _deployment_id = deployment_service.create(&new_deployment, &None).unwrap();
}
