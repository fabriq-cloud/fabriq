use uuid::Uuid;

pub mod common {
    tonic::include_proto!("akira.common");
}

pub use common::OperationId;

impl OperationId {
    pub fn create() -> Self {
        OperationId {
            id: Uuid::new_v4().to_string(),
        }
    }

    pub fn unwrap_or_create(current_operation_id: Option<OperationId>) -> OperationId {
        match current_operation_id {
            Some(current_operation_id) => current_operation_id,
            None => OperationId::create(),
        }
    }
}

// assignment protobufs

pub mod assignment {
    tonic::include_proto!("akira.assignment");
}

pub use assignment::assignment_server::{Assignment as AssignmentTrait, AssignmentServer};
pub use assignment::{
    AssignmentMessage, DeleteAssignmentRequest, ListAssignmentsRequest, ListAssignmentsResponse,
};

// deployment protobufs

pub mod deployment {
    tonic::include_proto!("akira.deployment");
}

pub use deployment::deployment_server::{Deployment as DeploymentTrait, DeploymentServer};
pub use deployment::{
    DeleteDeploymentRequest, DeploymentMessage, ListDeploymentsRequest, ListDeploymentsResponse,
};

// event protobufs

pub mod event {
    tonic::include_proto!("akira.event");
}

pub use event::{Event, EventType, ModelType};

impl From<i32> for EventType {
    fn from(event_type: i32) -> Self {
        match event_type {
            0 => EventType::Created,
            1 => EventType::Updated,
            2 => EventType::Deleted,
            _ => panic!("invalid event type"),
        }
    }
}

impl From<i32> for ModelType {
    fn from(event_type: i32) -> Self {
        match event_type {
            0 => ModelType::Assignment,
            1 => ModelType::Deployment,
            2 => ModelType::Host,
            3 => ModelType::Target,
            4 => ModelType::Template,
            5 => ModelType::Workload,
            6 => ModelType::Workspace,
            _ => panic!("invalid model type"),
        }
    }
}

// health protobufs

pub mod health {
    tonic::include_proto!("akira.health");
}

pub use health::health_server::{Health, HealthServer};
pub use health::{HealthRequest, HealthResponse};

// host protobufs

pub mod host {
    tonic::include_proto!("akira.host");
}

pub use host::host_server::{Host as HostTrait, HostServer};
pub use host::{DeleteHostRequest, HostMessage, ListHostsRequest, ListHostsResponse};

// target protobufs

pub mod target {
    tonic::include_proto!("akira.target");
}

pub use target::target_server::{Target as TargetTrait, TargetServer};
pub use target::{DeleteTargetRequest, ListTargetsRequest, ListTargetsResponse, TargetMessage};

// template protobufs

pub mod template {
    tonic::include_proto!("akira.template");
}

pub use template::template_server::{Template as TemplateTrait, TemplateServer};
pub use template::{
    DeleteTemplateRequest, ListTemplatesRequest, ListTemplatesResponse, TemplateMessage,
};

// workload protobufs

pub mod workload {
    tonic::include_proto!("akira.workload");
}

pub use workload::workload_server::{Workload as WorkloadTrait, WorkloadServer};
pub use workload::{
    DeleteWorkloadRequest, ListWorkloadsRequest, ListWorkloadsResponse, WorkloadMessage,
};

// workspace protobufs

pub mod workspace {
    tonic::include_proto!("akira.workspace");
}

pub use workspace::workspace_server::{Workspace as WorkspaceTrait, WorkspaceServer};
pub use workspace::{
    DeleteWorkspaceRequest, ListWorkspacesRequest, ListWorkspacesResponse, WorkspaceMessage,
};
