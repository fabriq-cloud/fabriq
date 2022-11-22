use std::time::SystemTime;

use prost_types::Timestamp;
use serde::{Deserialize, Serialize};
use urlencoding::decode;
use uuid::Uuid;

pub mod common {
    tonic::include_proto!("fabriq.common");
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
    tonic::include_proto!("fabriq.assignment");
}

pub use assignment::assignment_server::{Assignment as AssignmentTrait, AssignmentServer};
pub use assignment::{AssignmentMessage, ListAssignmentsRequest, ListAssignmentsResponse};

impl AssignmentMessage {
    pub const ASSIGNMENT_ID_SEPARATOR: char = '-';

    pub fn make_id(deployment_id: &str, host_id: &str) -> String {
        format!(
            "{deployment_id}{}{host_id}",
            AssignmentMessage::ASSIGNMENT_ID_SEPARATOR
        )
    }
}

// config protobufs

pub mod config {
    tonic::include_proto!("fabriq.config");
}

pub use config::config_server::{Config as ConfigTrait, ConfigServer};
pub use config::{ConfigIdRequest, ConfigMessage, QueryConfigRequest, QueryConfigResponse};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigValueType {
    StringType = 1,
    KeyValueType = 2,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigKeyValue {
    pub key: String,
    pub value: String,
}

impl ConfigMessage {
    pub const CONFIG_ID_SEPARATOR: char = ':';
    pub const OWNING_MODEL_SEPARATOR: char = ':';

    pub const DEPLOYMENT_OWNER: &str = "deployment";
    pub const TEMPLATE_OWNER: &str = "template";
    pub const WORKLOAD_OWNER: &str = "workload";

    pub fn make_id(owning_model: &str, key: &str) -> String {
        format!("{owning_model}{}{key}", ConfigMessage::CONFIG_ID_SEPARATOR)
    }

    pub fn make_owning_model(
        owning_model_type: &str,
        owning_model_id: &str,
    ) -> anyhow::Result<String> {
        match owning_model_type {
            ConfigMessage::WORKLOAD_OWNER
            | ConfigMessage::DEPLOYMENT_OWNER
            | ConfigMessage::TEMPLATE_OWNER => Ok(format!(
                "{owning_model_type}{}{owning_model_id}",
                ConfigMessage::OWNING_MODEL_SEPARATOR,
            )),
            _ => Err(anyhow::anyhow!(
                "unknown owning model type: {}",
                owning_model_type
            )),
        }
    }

    pub fn deserialize_keyvalue_pairs(&self) -> anyhow::Result<Vec<ConfigKeyValue>> {
        if self.value_type != ConfigValueType::KeyValueType as i32 {
            return Err(anyhow::anyhow!(
                "ConfigMessage::deserialize_subconfig: not KeyValue type"
            ));
        }

        let key_value_pairs = self.value.split(';').collect::<Vec<&str>>();

        let mut config_key_values = Vec::new();

        for key_value_pair in key_value_pairs {
            let kv_array = key_value_pair.split('=').collect::<Vec<&str>>();

            if kv_array.len() != 2 {
                return Err(anyhow::anyhow!(
                    "ConfigMessage::deserialize_subconfig: invalid key value pair: {}",
                    key_value_pair
                ));
            }

            let decoded_value = match decode(kv_array[1]) {
                Ok(value) => value.to_string(),
                Err(e) => return Err(anyhow::anyhow!(e)),
            };

            config_key_values.push(ConfigKeyValue {
                key: kv_array[0].to_string(),
                value: decoded_value,
            });
        }

        Ok(config_key_values)
    }
}

// deployment protobufs

pub mod deployment {
    tonic::include_proto!("fabriq.deployment");
}

pub use deployment::deployment_server::{Deployment as DeploymentTrait, DeploymentServer};
pub use deployment::{DeploymentMessage, ListDeploymentsRequest, ListDeploymentsResponse};

impl DeploymentMessage {
    const DEPLOYMENT_ID_SEPARATOR: char = '/';

    pub fn make_id(workload_id: &str, deployment_name: &str) -> String {
        format!(
            "{workload_id}{}{deployment_name}",
            DeploymentMessage::DEPLOYMENT_ID_SEPARATOR
        )
    }

    pub fn split_id(id: &str) -> anyhow::Result<(String, String, String)> {
        let id_parts = id
            .split(WorkloadMessage::TEAM_ID_SEPARATOR)
            .into_iter()
            .collect::<Vec<_>>();

        if id_parts.len() != 4 {
            return Err(anyhow::anyhow!("split_id: invalid deployment id"));
        }

        Ok((
            format!(
                "{}{}{}",
                id_parts[0],
                WorkloadMessage::TEAM_ID_SEPARATOR,
                id_parts[1]
            ),
            id_parts[2].to_string(),
            id_parts[3].to_string(),
        ))
    }
}

// event protobufs

pub mod event {
    tonic::include_proto!("fabriq.event");
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
        id: Uuid::new_v4().to_string(),
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

// host protobufs

pub mod host {
    tonic::include_proto!("fabriq.host");
}

pub use host::host_server::{Host as HostTrait, HostServer};
pub use host::{DeleteHostRequest, HostMessage, ListHostsRequest, ListHostsResponse};

// target protobufs

pub mod target {
    tonic::include_proto!("fabriq.target");
}

pub use target::target_server::{Target as TargetTrait, TargetServer};
pub use target::{ListTargetsRequest, ListTargetsResponse, TargetMessage};

// template protobufs

pub mod template {
    tonic::include_proto!("fabriq.template");
}

pub use template::template_server::{Template as TemplateTrait, TemplateServer};
pub use template::{ListTemplatesRequest, ListTemplatesResponse, TemplateMessage};

// workload protobufs

pub mod workload {
    tonic::include_proto!("fabriq.workload");
}

pub use workload::workload_server::{Workload as WorkloadTrait, WorkloadServer};
pub use workload::{ListWorkloadsRequest, ListWorkloadsResponse, WorkloadMessage};

impl WorkloadMessage {
    pub const TEAM_ID_SEPARATOR: char = '/';
    pub const WORKLOAD_ID_SEPARATOR: char = '/';

    pub fn make_id(team_id: &str, workload_name: &str) -> String {
        format!(
            "{team_id}{}{workload_name}",
            WorkloadMessage::WORKLOAD_ID_SEPARATOR
        )
    }

    pub fn make_team_id(org_id: &str, team_id: &str) -> String {
        format!("{org_id}{}{team_id}", WorkloadMessage::TEAM_ID_SEPARATOR)
    }

    pub fn split_team_id(team_id: &str) -> anyhow::Result<(String, String)> {
        let team_id_parts = team_id
            .split(WorkloadMessage::TEAM_ID_SEPARATOR)
            .into_iter()
            .collect::<Vec<_>>();

        if team_id_parts.len() != 2 {
            return Err(anyhow::anyhow!("invalid team id"));
        }

        Ok((team_id_parts[0].to_string(), team_id_parts[1].to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_split_id() -> anyhow::Result<()> {
        let team_id = WorkloadMessage::make_team_id("fabriq-cloud", "fabriq");
        assert_eq!(team_id, "fabriq-cloud/fabriq");

        let workload_id = WorkloadMessage::make_id(&team_id, "api");
        assert_eq!(workload_id, "fabriq-cloud/fabriq/api");

        let deployment_id = DeploymentMessage::make_id(&workload_id, "main");
        assert_eq!(deployment_id, "fabriq-cloud/fabriq/api/main");

        let result = DeploymentMessage::split_id(&team_id);
        assert!(result.is_err());

        let (team_id, workload_name, deployment_name) =
            DeploymentMessage::split_id(&deployment_id)?;

        assert_eq!(team_id, "fabriq-cloud/fabriq");
        assert_eq!(workload_name, "api");
        assert_eq!(deployment_name, "main");

        Ok(())
    }

    #[test]
    fn test_workload_split_team_id() -> anyhow::Result<()> {
        let team_id = WorkloadMessage::make_team_id("fabriq-cloud", "fabriq");

        assert_eq!(team_id, "fabriq-cloud/fabriq");

        let (org_id, team_id) = WorkloadMessage::split_team_id(&team_id)?;

        assert_eq!(org_id, "fabriq-cloud");
        assert_eq!(team_id, "fabriq");

        Ok(())
    }

    #[test]
    fn test_decode_kv() -> anyhow::Result<()> {
        let config = ConfigMessage {
            id: "test".to_owned(),
            key: "test".to_owned(),
            owning_model: "deployment:test".to_owned(),
            value: "A=postgres%3A%2F%2Fpostgres%3A%5Beuro4sure%5D%40fabriq.postgres.database.azure.com%2Ffabriq%3Fsslmode%3Drequire;B=postgres%3A%2F%2Fpostgres%3A%5Beuro4sure%5D%40fabriq.postgres.database.azure.com%2Ffabriq%3Fsslmode%3Drequire".to_owned(),
            value_type: ConfigValueType::KeyValueType as i32,
        };

        let kv = config.deserialize_keyvalue_pairs()?;

        assert_eq!(kv.len(), 2);
        assert_eq!(kv[0].key, "A");
        assert_eq!(kv[0].value, "postgres://postgres:[euro4sure]@fabriq.postgres.database.azure.com/fabriq?sslmode=require");
        assert_eq!(kv[1].key, "B");
        assert_eq!(kv[1].value, "postgres://postgres:[euro4sure]@fabriq.postgres.database.azure.com/fabriq?sslmode=require");

        Ok(())
    }
}
