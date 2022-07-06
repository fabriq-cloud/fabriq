use std::fmt::Debug;

use crate::models::{Assignment, Config, Deployment, Host, Target, Workload};

pub mod memory;
pub mod relational;

pub trait Persistence<Model>: Debug + Send + Sync {
    fn create(&self, model: &Model) -> anyhow::Result<String>;
    fn create_many(&self, models: &[Model]) -> anyhow::Result<Vec<String>>;
    fn delete(&self, model_id: &str) -> anyhow::Result<usize>;
    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize>;
    fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Model>>;
    fn list(&self) -> anyhow::Result<Vec<Model>>;
}

pub trait PersistableModel<Model>: Clone + Debug + Send + Sync {
    fn get_id(&self) -> String;
}

pub trait AssignmentPersistence: Debug + Send + Sync + Persistence<Assignment> {
    fn get_by_deployment_id(&self, id: &str) -> anyhow::Result<Vec<Assignment>>;
}

pub trait ConfigPersistence: Debug + Send + Sync + Persistence<Config> {
    fn get_by_deployment_id(&self, deployment_id: &str) -> anyhow::Result<Vec<Config>>;
    fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Config>>;
}

pub trait HostPersistence: Debug + Send + Sync + Persistence<Host> {
    fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>>;
}

pub trait DeploymentPersistence: Debug + Send + Sync + Persistence<Deployment> {
    fn get_by_target_id(&self, target_id: &str) -> anyhow::Result<Vec<Deployment>>;
    fn get_by_template_id(&self, id: &str) -> anyhow::Result<Vec<Deployment>>;
    fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Deployment>>;
}

pub trait WorkloadPersistence: Send + Sync + Persistence<Workload> {
    fn get_by_template_id(&self, id: &str) -> anyhow::Result<Vec<Workload>>;
}
