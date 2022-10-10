use tonic::{Request, Response, Status};

use crate::{
    ConfigIdRequest, ConfigMessage, ConfigTrait, ConfigValueType, DeploymentMessage, OperationId,
    QueryConfigRequest, QueryConfigResponse, WorkloadMessage,
};

pub struct MockConfigClient {}

#[tonic::async_trait]
impl ConfigTrait for MockConfigClient {
    async fn upsert(
        &self,
        _request: Request<ConfigMessage>,
    ) -> Result<Response<OperationId>, Status> {
        Ok(Response::new(OperationId::create()))
    }

    async fn delete(
        &self,
        _request: Request<ConfigIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        Ok(Response::new(OperationId::create()))
    }

    async fn query(
        &self,
        _request: Request<QueryConfigRequest>,
    ) -> Result<Response<QueryConfigResponse>, Status> {
        let workspace_id = "workspace-fixture";
        let workload_name = "workload-fixture";
        let workload_id = WorkloadMessage::make_id(workspace_id, workload_name);
        let deployment_name = "deployment-fixture";
        let deployment_id = DeploymentMessage::make_id(&workload_id, deployment_name);

        let configs = vec![
            ConfigMessage {
                id: ConfigMessage::make_id(&deployment_id, "replicas"),
                owning_model: ConfigMessage::make_owning_model("deployment", &deployment_id)
                    .unwrap(),
                key: "replicas".to_owned(),
                value: "5".to_owned(),

                value_type: ConfigValueType::StringType as i32,
            },
            ConfigMessage {
                id: ConfigMessage::make_id(&deployment_id, "labels"),
                owning_model: ConfigMessage::make_owning_model("deployment", &deployment_id)
                    .unwrap(),
                key: "labels".to_owned(),
                value: "cloud=azure;region=eastus2".to_owned(),

                value_type: ConfigValueType::KeyValueType as i32,
            },
            ConfigMessage {
                id: ConfigMessage::make_id(&workload_id, "port"),
                owning_model: ConfigMessage::make_owning_model("workload", &workload_id).unwrap(),
                key: "port".to_owned(),
                value: "80".to_owned(),

                value_type: ConfigValueType::StringType as i32,
            },
            ConfigMessage {
                id: ConfigMessage::make_id(&workload_id, "image"),
                owning_model: ConfigMessage::make_owning_model("deployment", &deployment_id)
                    .unwrap(),
                key: "image".to_owned(),
                value: "ghcr.io/timfpark/fabriq-gitops:aa14d4371cfc107bb5cc35d2cade57896841e0f9"
                    .to_owned(),

                value_type: ConfigValueType::StringType as i32,
            },
            ConfigMessage {
                id: ConfigMessage::make_id(&workload_id, "metricsEndpoint"),
                owning_model: ConfigMessage::make_owning_model("workload", &workload_id).unwrap(),
                key: "metricsEndpoint".to_owned(),
                value: "/metrics".to_owned(),

                value_type: ConfigValueType::StringType as i32,
            },
            ConfigMessage {
                id: ConfigMessage::make_id(&workload_id, "healthEndpoint"),
                owning_model: ConfigMessage::make_owning_model("workload", &workload_id).unwrap(),
                key: "healthEndpoint".to_owned(),
                value: "/healthz".to_owned(),

                value_type: ConfigValueType::StringType as i32,
            },
            ConfigMessage {
                id: ConfigMessage::make_id(&workload_id, "cpu"),
                owning_model: ConfigMessage::make_owning_model("workload", &workload_id).unwrap(),
                key: "cpu".to_owned(),
                value: "1000m".to_owned(),

                value_type: ConfigValueType::StringType as i32,
            },
            ConfigMessage {
                id: ConfigMessage::make_id(&workload_id, "memory"),
                owning_model: ConfigMessage::make_owning_model("workload", &workload_id).unwrap(),
                key: "memory".to_owned(),
                value: "128M".to_owned(),

                value_type: ConfigValueType::StringType as i32,
            },
        ];

        Ok(Response::new(QueryConfigResponse { configs }))
    }
}
