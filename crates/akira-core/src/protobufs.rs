use std::time::SystemTime;

use prost_types::Timestamp;
use uuid::Uuid;

pub mod common {
    tonic::include_proto!("akira.common");
}

pub use common::{DeploymentIdRequest, OperationId, TargetIdRequest, WorkloadIdRequest};

impl OperationId {
    pub fn create() -> Self {
        OperationId {
            id: Uuid::new_v4().to_string(),
        }
    }

    pub fn unwrap_or_create(current_operation_id: &Option<OperationId>) -> OperationId {
        match current_operation_id {
            Some(current_operation_id) => current_operation_id.clone(),
            None => OperationId::create(),
        }
    }
}

// assignment protobufs

pub mod assignment {
    tonic::include_proto!("akira.assignment");
}

pub use assignment::assignment_server::{Assignment as AssignmentTrait, AssignmentServer};
pub use assignment::{AssignmentMessage, ListAssignmentsRequest, ListAssignmentsResponse};

// config protobufs

pub mod config {
    tonic::include_proto!("akira.config");
}

pub use config::config_server::{Config as ConfigTrait, ConfigServer};
pub use config::{ConfigIdRequest, ConfigMessage, QueryConfigRequest, QueryConfigResponse};

// deployment protobufs

pub mod deployment {
    tonic::include_proto!("akira.deployment");
}

pub use deployment::deployment_server::{Deployment as DeploymentTrait, DeploymentServer};
pub use deployment::{DeploymentMessage, ListDeploymentsRequest, ListDeploymentsResponse};

// event protobufs

pub mod event {
    tonic::include_proto!("akira.event");
}

pub fn serialize_message_option<ModelMessage: prost::Message>(
    message_option: &Option<ModelMessage>,
) -> Option<Vec<u8>> {
    message_option
        .as_ref()
        .map(|message| message.encode_to_vec())
}

pub fn create_event<ModelMessage: prost::Message>(
    previous_model: &Option<ModelMessage>,
    current_model: &Option<ModelMessage>,
    event_type: EventType,
    model_type: ModelType,
    operation_id: &OperationId,
) -> Event {
    let serialized_previous_model = serialize_message_option::<ModelMessage>(previous_model);
    let serialized_current_model = serialize_message_option::<ModelMessage>(current_model);

    let timestamp = Timestamp {
        seconds: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        nanos: 0,
    };

    Event {
        operation_id: Some(operation_id.clone()),
        model_type: model_type as i32,
        serialized_previous_model,
        serialized_current_model,
        event_type: event_type as i32,
        timestamp: Some(timestamp),
    }
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
            7 => ModelType::Config,
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
pub use target::{ListTargetsRequest, ListTargetsResponse, TargetMessage};

// template protobufs

pub mod template {
    tonic::include_proto!("akira.template");
}

pub use template::template_server::{Template as TemplateTrait, TemplateServer};
pub use template::{ListTemplatesRequest, ListTemplatesResponse, TemplateMessage};

// workload protobufs

pub mod workload {
    tonic::include_proto!("akira.workload");
}

pub use workload::workload_server::{Workload as WorkloadTrait, WorkloadServer};
pub use workload::{ListWorkloadsRequest, ListWorkloadsResponse, WorkloadMessage};

// workspace protobufs

pub mod workspace {
    tonic::include_proto!("akira.workspace");
}

pub use workspace::workspace_server::{Workspace as WorkspaceTrait, WorkspaceServer};
pub use workspace::{
    DeleteWorkspaceRequest, ListWorkspacesRequest, ListWorkspacesResponse, WorkspaceMessage,
};
