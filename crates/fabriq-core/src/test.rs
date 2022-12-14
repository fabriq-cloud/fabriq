use crate::{
    AssignmentMessage, ConfigMessage, ConfigValueType, DeploymentMessage, HostMessage,
    TargetMessage, TemplateMessage, WorkloadMessage,
};

pub fn get_assignment_fixture(id: Option<&str>) -> AssignmentMessage {
    let deployment = get_deployment_fixture(None);
    let host = get_host_fixture(None);

    let generated_assignment_id = AssignmentMessage::make_id(&deployment.id, &host.id);
    let id = id.unwrap_or(&generated_assignment_id).to_string();

    AssignmentMessage {
        id,
        host_id: host.id,
        deployment_id: deployment.id,
    }
}

pub fn get_keyvalue_config_fixture() -> ConfigMessage {
    let deployment = get_deployment_fixture(None);

    ConfigMessage {
        id: "config-string-fixture".to_owned(),

        owning_model: ConfigMessage::make_owning_model("deployment", &deployment.id).unwrap(),
        key: "config".to_owned(),
        value: "key1=value1;key2=value2".to_owned(),

        value_type: ConfigValueType::KeyValueType as i32,
    }
}

pub fn get_string_config_fixture() -> ConfigMessage {
    let workload = get_workload_fixture(None);

    ConfigMessage {
        id: "config-keyvalue-fixture".to_owned(),

        owning_model: ConfigMessage::make_owning_model("workload", &workload.id).unwrap(),
        key: "cpu".to_owned(),
        value: "100m".to_owned(),

        value_type: ConfigValueType::StringType as i32,
    }
}

pub fn get_deployment_fixture(name: Option<&str>) -> DeploymentMessage {
    let target = get_target_fixture(None);
    let template = get_template_fixture(None);

    let workload = get_workload_fixture(None);
    let deployment_name = name.unwrap_or("deployment-fixture");
    let deployment_id = DeploymentMessage::make_id(&workload.id, deployment_name);

    DeploymentMessage {
        id: deployment_id,
        name: deployment_name.to_string(),
        target_id: target.id,
        workload_id: workload.id,
        template_id: Some(template.id),
        host_count: 2,
    }
}

pub fn get_host_fixture(name: Option<&str>) -> HostMessage {
    let id = name.unwrap_or("host-fixture").to_string();

    HostMessage {
        id,
        labels: vec!["region:eastus2".to_string(), "cloud:azure".to_string()],
    }
}

pub fn get_target_fixture(name: Option<&str>) -> TargetMessage {
    let id = name.unwrap_or("target-fixture").to_string();

    TargetMessage {
        id,
        labels: vec!["region:eastus2".to_string()],
    }
}

pub fn get_template_fixture(name: Option<&str>) -> TemplateMessage {
    let id = name.unwrap_or("template-fixture").to_string();

    TemplateMessage {
        id,
        repository: "git@github.com:timfpark/deployment-templates".to_owned(),
        git_ref: "main".to_owned(),
        path: "external-service".to_owned(),
    }
}

pub fn get_workload_fixture(name: Option<&str>) -> WorkloadMessage {
    let workload_name = name.unwrap_or("workload-fixture").to_string();
    let template = get_template_fixture(None);
    let team_id = get_team_fixture();

    let workload_id = WorkloadMessage::make_id(&team_id, &workload_name);

    WorkloadMessage {
        id: workload_id,
        name: workload_name,
        template_id: template.id,
        team_id,
    }
}

pub fn get_team_fixture() -> String {
    // NOTE: Need to use something real here because we use the Github API to query it.
    WorkloadMessage::make_team_id("fabriq-cloud", "fabriq")
}
