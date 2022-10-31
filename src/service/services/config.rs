use fabriq_core::{
    create_event, ConfigMessage, EventStream, EventType, ModelType, OperationId, QueryConfigRequest,
};
use std::{collections::HashMap, sync::Arc};

use crate::{models::Config, persistence::ConfigPersistence};

use super::{DeploymentService, WorkloadService};

#[derive(Debug)]
pub struct ConfigService {
    pub persistence: Box<dyn ConfigPersistence>,
    pub event_stream: Arc<dyn EventStream>,

    pub deployment_service: Arc<DeploymentService>,
    pub workload_service: Arc<WorkloadService>,
}

impl ConfigService {
    #[tracing::instrument(name = "service::config::create")]
    pub async fn upsert(
        &self,
        config: &Config,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let affected_count = self.persistence.upsert(config).await?;
        let operation_id = OperationId::unwrap_or_create(operation_id);

        if affected_count > 0 {
            let create_event = create_event::<ConfigMessage>(
                &None,
                &Some(config.clone().into()),
                EventType::Created,
                ModelType::Config,
                &operation_id,
            );

            self.event_stream.send(&create_event).await?;
        }

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::config::get_by_id")]
    pub async fn get_by_id(&self, config_id: &str) -> anyhow::Result<Option<Config>> {
        self.persistence.get_by_id(config_id).await
    }

    #[tracing::instrument(name = "service::config::delete")]
    pub async fn delete(
        &self,
        config_id: &str,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let config = match self.get_by_id(config_id).await? {
            Some(config) => config,
            None => return Err(anyhow::anyhow!("Config id {config_id} not found")),
        };

        let deleted_count = self.persistence.delete(config_id).await?;

        if deleted_count == 0 {
            return Err(anyhow::anyhow!("Config id {config_id} not found"));
        }

        let operation_id = OperationId::unwrap_or_create(operation_id);

        let delete_event = create_event::<ConfigMessage>(
            &Some(config.clone().into()),
            &None,
            EventType::Deleted,
            ModelType::Config,
            &operation_id,
        );

        self.event_stream.send(&delete_event).await?;

        tracing::info!("config deleted: {:?}", config);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::config::query")]
    pub async fn query(&self, query: &QueryConfigRequest) -> anyhow::Result<Vec<Config>> {
        let model_name = query.model_name.as_str();
        let mut config_set = HashMap::new();

        let template_config;
        let mut deployment_config = vec![];
        let mut workload_config = vec![];

        match model_name {
            "deployment" => {
                let deployment = match self.deployment_service.get_by_id(&query.model_id).await? {
                    Some(deployment) => deployment,
                    None => {
                        return Err(anyhow::anyhow!(
                            "Deployment id {} not found",
                            query.model_id
                        ));
                    }
                };

                deployment_config = self
                    .persistence
                    .get_by_deployment_id(&deployment.id)
                    .await?;

                let workload = match self
                    .workload_service
                    .get_by_id(&deployment.workload_id)
                    .await?
                {
                    Some(workload) => workload,
                    None => {
                        return Err(anyhow::anyhow!(
                            "workload id {} not found",
                            deployment.workload_id
                        ));
                    }
                };

                if let Some(template_id) = deployment.template_id {
                    template_config = self.persistence.get_by_template_id(&template_id).await?;
                } else {
                    template_config = self
                        .persistence
                        .get_by_template_id(&workload.template_id)
                        .await?;
                }

                workload_config = self
                    .persistence
                    .get_by_workload_id(&deployment.workload_id)
                    .await?;
            }

            "workload" => {
                let workload = match self.workload_service.get_by_id(&query.model_id).await? {
                    Some(deployment) => deployment,
                    None => {
                        return Err(anyhow::anyhow!(
                            "Deployment id {} not found",
                            query.model_id
                        ));
                    }
                };

                workload_config = self.persistence.get_by_workload_id(&query.model_id).await?;
                template_config = self
                    .persistence
                    .get_by_template_id(&workload.template_id)
                    .await?;
            }

            "template" => {
                template_config = self.persistence.get_by_template_id(&query.model_id).await?;
            }
            _ => return Err(anyhow::anyhow!("Model type not supported")),
        }

        // shred config in tiered order into a HashMap such that deployment config overrides
        // workload config overrides template config.

        for config in template_config {
            config_set.insert(config.key.clone(), config);
        }

        for config in workload_config {
            config_set.insert(config.key.clone(), config);
        }

        for config in deployment_config {
            config_set.insert(config.key.clone(), config);
        }

        Ok(config_set.values().cloned().collect())
    }

    #[tracing::instrument(name = "service::config::get_by_deployment_id")]
    pub async fn _get_by_deployment_id(&self, deployment_id: &str) -> anyhow::Result<Vec<Config>> {
        self.persistence.get_by_deployment_id(deployment_id).await
    }

    #[tracing::instrument(name = "service::config::get_by_workload_id")]
    pub async fn _get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Config>> {
        self.persistence.get_by_workload_id(workload_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{Deployment, Target, Template, Workload},
        persistence::memory::{
            ConfigMemoryPersistence, DeploymentMemoryPersistence, MemoryPersistence,
            WorkloadMemoryPersistence,
        },
        services::{TargetService, TemplateService},
    };
    use fabriq_core::test::{
        get_deployment_fixture, get_string_config_fixture, get_target_fixture,
        get_template_fixture, get_workload_fixture,
    };
    use fabriq_memory_stream::MemoryEventStream;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();

        let config_persistence = ConfigMemoryPersistence::default();
        let event_stream: Arc<dyn EventStream> = Arc::new(MemoryEventStream::new().unwrap());

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

            template_service,
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
        target_service.upsert(&target, &None).await.unwrap();

        let deployment_persistence = Box::new(DeploymentMemoryPersistence::default());
        let deployment_service = Arc::new(DeploymentService {
            event_stream: Arc::clone(&event_stream) as Arc<dyn EventStream>,
            persistence: deployment_persistence,

            target_service,
        });

        let deployment: Deployment = get_deployment_fixture(None).into();
        deployment_service.upsert(&deployment, &None).await.unwrap();

        let config_service = ConfigService {
            persistence: Box::new(config_persistence),
            event_stream,

            deployment_service,
            workload_service,
        };

        let config: Config = get_string_config_fixture().into();

        let config_created_operation_id = config_service
            .upsert(&config, &Some(OperationId::create()))
            .await
            .unwrap();
        assert_eq!(config_created_operation_id.id.len(), 36);

        let fetched_config = config_service.get_by_id(&config.id).await.unwrap().unwrap();
        assert_eq!(fetched_config.id, config.id);

        let configs_by_workload = config_service
            ._get_by_workload_id(&workload.id)
            .await
            .unwrap();
        assert_eq!(configs_by_workload.len(), 1);

        let query = QueryConfigRequest {
            model_name: "deployment".to_string(),
            model_id: deployment.id,
        };

        let config_for_deployment = config_service.query(&query).await.unwrap();
        assert_eq!(config_for_deployment.len(), 1);

        let deleted_operation_id = config_service.delete(&config.id, &None).await.unwrap();
        assert_eq!(deleted_operation_id.id.len(), 36);
    }
}
