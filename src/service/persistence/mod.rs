use crate::models::{Assignment, Deployment, Host, Target};

pub mod memory;
pub mod relational;

pub trait Persistence<Model>: Send + Sync {
    fn create(&self, model: &Model) -> anyhow::Result<String>;
    fn create_many(&self, models: &[Model]) -> anyhow::Result<Vec<String>>;
    fn delete(&self, model_id: &str) -> anyhow::Result<usize>;
    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize>;
    fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Model>>;
    fn list(&self) -> anyhow::Result<Vec<Model>>;
}

pub trait PersistableModel<Model> {
    fn get_id(&self) -> String;
}

pub trait AssignmentPersistence: Send + Sync + Persistence<Assignment> {
    fn get_by_deployment_id(&self, id: &str) -> anyhow::Result<Vec<Assignment>>;
}

pub trait HostPersistence: Send + Sync + Persistence<Host> {
    fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>>;
}

pub trait DeploymentPersistence: Send + Sync + Persistence<Deployment> {
    fn get_by_target_id(&self, target_id: &str) -> anyhow::Result<Vec<Deployment>>;
}
