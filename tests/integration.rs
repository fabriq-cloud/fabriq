use std::sync::Arc;

use akira::{
    models::{Deployment, Host, Target, Template, Workload, Workspace},
    persistence::memory::MemoryPersistence,
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

    let host_persistence = MemoryPersistence::<Host, Host>::default();

    let cloned_event_stream = Arc::clone(&event_stream);
    let host_service = HostService::new(Box::new(host_persistence), cloned_event_stream);

    // create a host
    let new_host = Host {
        id: "azure-eastus2-1".to_owned(),
        labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],

        cpu_capacity: 4000,
        memory_capacity: 24000,
    };

    host_service
        .create(new_host, Some(OperationId::create()))
        .await
        .unwrap();

    let target_persistence = MemoryPersistence::<Target, Target>::default();

    let cloned_event_stream = Arc::clone(&event_stream);
    let target_service = TargetService::new(Box::new(target_persistence), cloned_event_stream);

    // create target that matches host
    let new_target = Target {
        id: "eastus2".to_string(),
        labels: vec!["location:eastus2".to_string()],
    };

    let create_target_operation_id = target_service
        .create(new_target.clone(), None)
        .await
        .unwrap();

    assert_eq!(create_target_operation_id.id.len(), 36);

    let template_persistence = MemoryPersistence::<Template, Template>::default();

    let cloned_event_stream = Arc::clone(&event_stream);
    let template_service =
        TemplateService::new(Box::new(template_persistence), cloned_event_stream);

    // create template
    let new_template = Template {
        id: "external-service".to_owned(),
        repository: "http://github.com/timfpark/deployment-templates".to_owned(),
        branch: "main".to_owned(),
        path: "external-service".to_owned(),
    };

    let create_template_operation_id = template_service
        .create(new_template.clone(), Some(OperationId::create()))
        .await
        .unwrap();

    assert_eq!(create_template_operation_id.id.len(), 36);

    let workload_persistence = Box::new(MemoryPersistence::<Workload, Workload>::default());
    let workload_service = Arc::new(WorkloadService {
        persistence: workload_persistence,
        event_stream: Arc::clone(&event_stream),
    });

    let workspace_persistence = Box::new(MemoryPersistence::<Workspace, Workspace>::default());
    let workspace_service = WorkspaceService {
        persistence: workspace_persistence,
        event_stream: Arc::clone(&event_stream),

        workload_service: Arc::clone(&workload_service),
    };

    // create workspace
    let new_workspace = Workspace {
        id: "foreign-exchange".to_string(),
    };

    let create_workspace_operation_id = workspace_service
        .create(new_workspace.clone(), None)
        .await
        .unwrap();

    assert_eq!(create_workspace_operation_id.id.len(), 36);

    // create workload
    let new_workload = Workload {
        id: "cribbage-api".to_string(),
        workspace_id: new_workspace.id,
        template_id: new_template.id,
    };

    let create_workload_operation_id = workload_service
        .create(new_workload.clone(), Some(OperationId::create()))
        .await
        .unwrap();

    assert_eq!(create_workload_operation_id.id.len(), 36);

    let deployment_persistence = MemoryPersistence::<Deployment, Deployment>::default();
    let cloned_event_stream = Arc::clone(&event_stream);

    let deployment_service =
        DeploymentService::new(Box::new(deployment_persistence), cloned_event_stream);

    // create deployment
    let new_deployment = Deployment {
        id: "cribbage-api-prod".to_string(),
        workload_id: new_workload.id,
        target_id: new_target.id,
        replicas: 3,
    };

    let _deployment_id = deployment_service
        .create(new_deployment, None)
        .await
        .unwrap();
}