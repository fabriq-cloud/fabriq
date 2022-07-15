use akira_core::{
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
    pub fn create(
        &self,
        config: &Config,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        self.persistence.create(config)?;

        let operation_id = OperationId::unwrap_or_create(operation_id);
        let create_event = create_event::<ConfigMessage>(
            &None,
            &Some(config.clone().into()),
            EventType::Created,
            ModelType::Config,
            &operation_id,
        );

        self.event_stream.send(&create_event)?;

        tracing::info!("config created: {:?}", config);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::config::get_by_id")]
    pub fn get_by_id(&self, config_id: &str) -> anyhow::Result<Option<Config>> {
        self.persistence.get_by_id(config_id)
    }

    #[tracing::instrument(name = "service::config::d&elete")]
    pub fn delete(
        &self,
        config_id: &str,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let config = match self.get_by_id(config_id)? {
            Some(config) => config,
            None => return Err(anyhow::anyhow!("Config id {config_id} not found")),
        };

        let deleted_count = self.persistence.delete(config_id)?;

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

        self.event_stream.send(&delete_event)?;

        tracing::info!("config deleted: {:?}", config);

        Ok(operation_id)
    }

    #[tracing::instrument(name = "service::config::query")]
    pub fn query(&self, query: &QueryConfigRequest) -> anyhow::Result<Vec<Config>> {
        let model_name = query.model_name.as_str();
        let mut config_set = HashMap::new();

        let template_config;
        let mut deployment_config = vec![];
        let mut workload_config = vec![];

        match model_name {
            "deployment" => {
                println!("deployment");

                let deployment = match self.deployment_service.get_by_id(&query.model_id)? {
                    Some(deployment) => deployment,
                    None => {
                        return Err(anyhow::anyhow!(
                            "Deployment id {} not found",
                            query.model_id
                        ));
                    }
                };

                println!("deployment: {:#?}", deployment);

                deployment_config = self.persistence.get_by_deployment_id(&deployment.id)?;

                let workload = match self.workload_service.get_by_id(&deployment.workload_id)? {
                    Some(workload) => workload,
                    None => {
                        return Err(anyhow::anyhow!(
                            "workload id {} not found",
                            deployment.workload_id
                        ));
                    }
                };

                println!("deployment_config: {:#?}", deployment_config);

                if let Some(template_id) = deployment.template_id {
                    template_config = self.persistence.get_by_template_id(&template_id)?;
                } else {
                    template_config = self.persistence.get_by_template_id(&workload.template_id)?;
                }

                println!("template_config: {:#?}", template_config);

                workload_config = self
                    .persistence
                    .get_by_workload_id(&deployment.workload_id)?;

                println!("workload_config: {:#?}", workload_config);
            }

            "workload" => {
                let workload = match self.workload_service.get_by_id(&query.model_id)? {
                    Some(deployment) => deployment,
                    None => {
                        return Err(anyhow::anyhow!(
                            "Deployment id {} not found",
                            query.model_id
                        ));
                    }
                };

                workload_config = self.persistence.get_by_workload_id(&query.model_id)?;
                template_config = self.persistence.get_by_template_id(&workload.template_id)?;
            }

            "template" => {
                template_config = self.persistence.get_by_template_id(&query.model_id)?;
            }
            _ => return Err(anyhow::anyhow!("Model type not supported")),
        }

        // shred config in tiered order into a HashMap such that deployment config overrides
        // workload config overrides template config.

        for config in template_config {
            config_set.insert(config.id.clone(), config);
        }

        for config in workload_config {
            config_set.insert(config.id.clone(), config);
        }

        for config in deployment_config {
            config_set.insert(config.id.clone(), config);
        }

        Ok(config_set.values().cloned().collect())
    }

    #[tracing::instrument(name = "service::config::get_by_deployment_id")]
    pub fn get_by_deployment_id(&self, deployment_id: &str) -> anyhow::Result<Vec<Config>> {
        self.persistence.get_by_deployment_id(deployment_id)
    }

    #[tracing::instrument(name = "service::config::get_by_workload_id")]
    pub fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Config>> {
        self.persistence.get_by_workload_id(workload_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{Deployment, Workload},
        persistence::memory::{
            ConfigMemoryPersistence, DeploymentMemoryPersistence, WorkloadMemoryPersistence,
        },
    };
    use akira_core::test::{
        get_deployment_fixture, get_string_config_fixture, get_workload_fixture,
    };
    use akira_memory_stream::MemoryEventStream;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let config_persistence = ConfigMemoryPersistence::default();
        let event_stream = Arc::new(MemoryEventStream::new().unwrap());

        let workload_persistence = Box::new(WorkloadMemoryPersistence::default());
        let workload_service = Arc::new(WorkloadService {
            event_stream: Arc::clone(&event_stream) as Arc<dyn EventStream>,
            persistence: workload_persistence,
        });

        let workload: Workload = get_workload_fixture(None).into();
        workload_service.create(&workload, None).unwrap();

        let deployment_persistence = Box::new(DeploymentMemoryPersistence::default());
        let deployment_service = Arc::new(DeploymentService {
            event_stream: Arc::clone(&event_stream) as Arc<dyn EventStream>,
            persistence: deployment_persistence,
        });

        let deployment: Deployment = get_deployment_fixture(None).into();
        deployment_service.create(&deployment, &None).unwrap();

        let config_service = ConfigService {
            persistence: Box::new(config_persistence),
            event_stream,

            deployment_service,
            workload_service,
        };

        let config: Config = get_string_config_fixture().into();

        let config_created_operation_id = config_service
            .create(&config, &Some(OperationId::create()))
            .unwrap();
        assert_eq!(config_created_operation_id.id.len(), 36);

        let fetched_config = config_service.get_by_id(&config.id).unwrap().unwrap();
        assert_eq!(fetched_config.id, config.id);

        let configs_by_workload = config_service.get_by_workload_id(&workload.id).unwrap();
        assert_eq!(configs_by_workload.len(), 1);

        let query = QueryConfigRequest {
            model_name: "deployment".to_string(),
            model_id: deployment.id,
        };

        let config_for_deployment = config_service.query(&query).unwrap();
        assert_eq!(config_for_deployment.len(), 1);

        let deleted_operation_id = config_service.delete(&config.id, &None).unwrap();
        assert_eq!(deleted_operation_id.id.len(), 36);
    }
}
