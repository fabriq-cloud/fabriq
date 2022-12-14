use fabriq_core::{
    ConfigIdRequest, ConfigMessage, ConfigTrait, OperationId, QueryConfigRequest,
    QueryConfigResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use super::auth::*;
use crate::models::{Config, Deployment, Workload};
use crate::services::{ConfigService, DeploymentService, WorkloadService};

#[derive(Debug)]
pub struct GrpcConfigService {
    pub config_service: Arc<ConfigService>,
    pub deployment_service: Arc<DeploymentService>,
    pub workload_service: Arc<WorkloadService>,
}

impl GrpcConfigService {
    async fn wrapped_get_deployment_by_id(
        &self,
        deployment_id: &str,
    ) -> Result<Deployment, Status> {
        let deployment_result = match self.deployment_service.get_by_id(deployment_id).await {
            Ok(deployment) => match deployment {
                Some(deployment) => deployment,
                None => {
                    return Err(Status::new(
                        tonic::Code::NotFound,
                        format!("deployment with id {} not found", deployment_id),
                    ))
                }
            },
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("get_by_id with id {} failed", err),
                ))
            }
        };

        Ok(deployment_result)
    }

    async fn wrapped_get_workload_by_id(&self, workload_id: &str) -> Result<Workload, Status> {
        let workload_result = match self.workload_service.get_by_id(workload_id).await {
            Ok(deployment) => match deployment {
                Some(deployment) => deployment,
                None => {
                    return Err(Status::new(
                        tonic::Code::NotFound,
                        format!("deployment with id {} not found", workload_id),
                    ))
                }
            },
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("get_by_id with id {} failed", err),
                ))
            }
        };

        Ok(workload_result)
    }

    async fn check_auth(&self, pat: &str, config: &Config) -> Result<(), Status> {
        let (owning_model, owning_model_id) = match config.split_owning_model() {
            Ok((owning_model, owning_model_id)) => (owning_model, owning_model_id),
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::InvalidArgument,
                    format!("split_owning_model failed: {}", err),
                ))
            }
        };

        if owning_model == ConfigMessage::DEPLOYMENT_OWNER {
            let deployment = match self.wrapped_get_deployment_by_id(&owning_model_id).await {
                Ok(deployment) => deployment,
                Err(err) => {
                    return Err(Status::new(
                        tonic::Code::InvalidArgument,
                        format!("get_deployment_by_id failed: {}", err),
                    ))
                }
            };

            let workload = match self
                .wrapped_get_workload_by_id(&deployment.workload_id)
                .await
            {
                Ok(workload) => workload,
                Err(err) => {
                    return Err(Status::new(
                        tonic::Code::InvalidArgument,
                        format!("get_workload_by_id failed: {}", err),
                    ))
                }
            };

            is_team_member(pat, &workload.team_id).await?;
        } else if owning_model == ConfigMessage::WORKLOAD_OWNER {
            let workload = match self.wrapped_get_workload_by_id(&owning_model_id).await {
                Ok(workload) => workload,
                Err(err) => {
                    return Err(Status::new(
                        tonic::Code::InvalidArgument,
                        format!("get_workload_by_id failed: {}", err),
                    ))
                }
            };

            is_team_member(pat, &workload.team_id).await?;
        } else if owning_model == ConfigMessage::TEMPLATE_OWNER {
            // what to do here?  check if member of special platform team?
            // should templates be owned by a team so changes can be authed?
        } else {
            return Err(Status::new(
                tonic::Code::InvalidArgument,
                "owning_model type is unknown".to_string(),
            ));
        }

        Ok(())
    }
}

#[tonic::async_trait]
impl ConfigTrait for GrpcConfigService {
    #[tracing::instrument(name = "grpc::config::upsert")]
    async fn upsert(
        &self,
        request: Request<ConfigMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let pat = crate::acl::get_pat_from_headers(&request).await?;
        let config: Config = request.into_inner().into();

        self.check_auth(&pat, &config).await?;

        let operation_id = match self.config_service.upsert(&config, &None).await {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::InvalidArgument,
                    format!("config could not be upserted: {err}"),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    #[tracing::instrument(name = "grpc::config::delete")]
    async fn delete(
        &self,
        request: Request<ConfigIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        let pat = crate::acl::get_pat_from_headers(&request).await?;
        let config_id = request.into_inner().config_id;

        let config = match self.config_service.get_by_id(&config_id).await {
            Ok(config) => match config {
                Some(config) => config,
                None => {
                    return Err(Status::new(
                        tonic::Code::NotFound,
                        format!("config with id {config_id} not found"),
                    ))
                }
            },
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("fetching config with id {config_id} failed with {err}"),
                ))
            }
        };

        self.check_auth(&pat, &config).await?;

        // TODO: check that no workloads are currently still using config
        // Query workload service for workloads by config_id, error if any exist

        let operation_id = match self.config_service.delete(&config_id, &None).await {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("config with id {} not found", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    #[tracing::instrument(name = "grpc::config::query")]
    async fn query(
        &self,
        request: Request<QueryConfigRequest>,
    ) -> Result<Response<QueryConfigResponse>, Status> {
        let query = request.into_inner();

        let (deployment, workload, template_id) = match query.model_name.as_ref() {
            ConfigMessage::DEPLOYMENT_OWNER => {
                let deployment = self.wrapped_get_deployment_by_id(&query.model_id).await?;
                let workload = self
                    .wrapped_get_workload_by_id(&deployment.workload_id)
                    .await?;
                let template_id = deployment
                    .template_id
                    .clone()
                    .unwrap_or_else(|| workload.template_id.clone());

                (Some(deployment), Some(workload), template_id)
            }

            ConfigMessage::TEMPLATE_OWNER => (None, None, query.model_id.clone()),

            ConfigMessage::WORKLOAD_OWNER => {
                let workload = self.wrapped_get_workload_by_id(&query.model_id).await?;
                let template_id = workload.template_id.clone();

                (None, Some(workload), template_id)
            }

            _ => {
                return Err(Status::new(
                    tonic::Code::InvalidArgument,
                    format!("unknown query model owner type: {}", query.model_name),
                ))
            }
        };

        let configs = match self
            .config_service
            .query(&query, deployment, workload, &template_id)
            .await
        {
            Ok(configs) => configs,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("config query failed with {}", err),
                ))
            }
        };

        let config_messages = configs.iter().map(|config| config.clone().into()).collect();

        let response = QueryConfigResponse {
            configs: config_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use fabriq_core::{
        test::{
            get_deployment_fixture, get_string_config_fixture, get_target_fixture,
            get_template_fixture, get_workload_fixture,
        },
        ConfigIdRequest, ConfigTrait, EventStream, QueryConfigRequest,
    };
    use fabriq_memory_stream::MemoryEventStream;
    use std::{env, sync::Arc};
    use tonic::{metadata::MetadataValue, Request};

    use super::GrpcConfigService;

    use crate::services::{
        ConfigService, DeploymentService, TargetService, TemplateService, WorkloadService,
    };
    use crate::{
        models::{Deployment, Target, Template, Workload},
        persistence::memory::AssignmentMemoryPersistence,
    };
    use crate::{
        persistence::memory::{
            ConfigMemoryPersistence, DeploymentMemoryPersistence, MemoryPersistence,
            WorkloadMemoryPersistence,
        },
        services::AssignmentService,
    };

    async fn create_config_grpc_service() -> GrpcConfigService {
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let template_persistence = MemoryPersistence::<Template>::default();
        let template_service = Arc::new(TemplateService {
            persistence: Box::new(template_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let template: Template = get_template_fixture(Some("template-fixture")).into();
        let operation_id = template_service.upsert(&template, None).await.unwrap();

        let workload_persistence = Box::new(WorkloadMemoryPersistence::default());
        let workload_service = Arc::new(WorkloadService {
            event_stream: Arc::clone(&event_stream) as Arc<dyn EventStream>,
            persistence: workload_persistence,

            template_service: Arc::clone(&template_service),
        });

        let workload: Workload = get_workload_fixture(None).into();
        workload_service
            .upsert(&workload, Some(operation_id))
            .await
            .unwrap();

        let target_persistence = MemoryPersistence::<Target>::default();
        let target_service = Arc::new(TargetService {
            persistence: Box::new(target_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let target: Target = get_target_fixture(None).into();
        let operation_id = target_service.upsert(&target, &None).await.unwrap();

        let assignment_persistence = Box::new(AssignmentMemoryPersistence::default());
        let assignment_service = Arc::new(AssignmentService {
            persistence: assignment_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let config_persistence = Box::new(ConfigMemoryPersistence::default());
        let config_service = Arc::new(ConfigService {
            persistence: config_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let deployment_persistence = Box::new(DeploymentMemoryPersistence::default());
        let deployment_service = Arc::new(DeploymentService {
            event_stream: Arc::clone(&event_stream),
            persistence: deployment_persistence,

            assignment_service: Arc::clone(&assignment_service),
            config_service: Arc::clone(&config_service),
            target_service,
        });

        let deployment: Deployment = get_deployment_fixture(None).into();

        deployment_service
            .upsert(&deployment, &Some(operation_id))
            .await
            .unwrap();

        GrpcConfigService {
            config_service: Arc::clone(&config_service),
            deployment_service: Arc::clone(&deployment_service),
            workload_service: Arc::clone(&workload_service),
        }
    }
    #[tokio::test]
    async fn test_check_auth() -> anyhow::Result<()> {
        let access_token =
            env::var("FABRIQ_GITHUB_TOKEN").expect("FABRIQ_GITHUB_TOKEN must be set");
        let config = get_string_config_fixture().into();

        let config_grpc_service = create_config_grpc_service().await;

        config_grpc_service
            .check_auth(&access_token, &config)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_create_list_config() -> anyhow::Result<()> {
        let config = get_string_config_fixture();
        let config_grpc_service = create_config_grpc_service().await;

        let deployment: Deployment = get_deployment_fixture(None).into();

        let mut request = Request::new(config.clone());

        let access_token =
            env::var("FABRIQ_GITHUB_TOKEN").expect("FABRIQ_GITHUB_TOKEN must be set");
        let token: MetadataValue<_> = access_token.parse()?;

        request
            .metadata_mut()
            .insert("authorization", token.clone());

        let response = config_grpc_service
            .upsert(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(QueryConfigRequest {
            model_name: "deployment".to_string(),
            model_id: deployment.id.clone(),
        });

        let response = config_grpc_service
            .query(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.configs.len(), 1);

        let mut request = Request::new(ConfigIdRequest {
            config_id: config.id.clone(),
        });

        request.metadata_mut().insert("authorization", token);

        let response = config_grpc_service
            .delete(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        Ok(())
    }
}
