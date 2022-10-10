use async_trait::async_trait;
use std::fmt::Debug;

use crate::models::{Assignment, Config, Deployment, Host, Target, Workload};

pub mod memory;
pub mod relational;

#[async_trait]
pub trait Persistence<Model>: Debug + Send + Sync {
    async fn upsert(&self, model: &Model) -> anyhow::Result<u64>;
    async fn delete(&self, model_id: &str) -> anyhow::Result<u64>;
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Model>>;
    async fn list(&self) -> anyhow::Result<Vec<Model>>;
}

pub trait Persistable<Model>: Clone + Debug + Send + Sync {
    fn get_id(&self) -> String;
}

#[async_trait]
pub trait AssignmentPersistence: Debug + Send + Sync + Persistence<Assignment> {
    async fn get_by_deployment_id(&self, id: &str) -> anyhow::Result<Vec<Assignment>>;
}

#[async_trait]
pub trait ConfigPersistence: Debug + Send + Sync + Persistence<Config> {
    async fn get_by_deployment_id(&self, deployment_id: &str) -> anyhow::Result<Vec<Config>>;
    async fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Config>>;
    async fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Config>>;
}

#[async_trait]
pub trait DeploymentPersistence: Debug + Send + Sync + Persistence<Deployment> {
    async fn get_by_target_id(&self, target_id: &str) -> anyhow::Result<Vec<Deployment>>;
    async fn get_by_template_id(&self, id: &str) -> anyhow::Result<Vec<Deployment>>;
    async fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Deployment>>;
}

#[async_trait]
pub trait HostPersistence: Debug + Send + Sync + Persistence<Host> {
    async fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>>;
}

#[async_trait]
pub trait TargetPersistence: Debug + Send + Sync + Persistence<Target> {
    async fn get_matching_host(&self, host: &Host) -> anyhow::Result<Vec<Target>>;
}

#[async_trait]
pub trait WorkloadPersistence: Send + Sync + Persistence<Workload> {
    async fn get_by_template_id(&self, id: &str) -> anyhow::Result<Vec<Workload>>;
}
