use async_trait::async_trait;

use crate::models::{Assignment, Host, Target};

pub mod memory;
pub mod relational;

#[async_trait]
pub trait AssignmentPersistence: Send + Sync {
    async fn create(&self, assignment: &Assignment) -> anyhow::Result<String>;
    async fn delete(&self, assignment_id: &str) -> anyhow::Result<usize>;
    async fn list(&self) -> anyhow::Result<Vec<Assignment>>;
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Assignment>>;
    async fn get_by_deployment_id(&self, id: &str) -> anyhow::Result<Vec<Assignment>>;
}

#[async_trait]
pub trait HostPersistence: Send + Sync {
    async fn create(&self, host: &Host) -> anyhow::Result<String>;
    async fn delete(&self, host_id: &str) -> anyhow::Result<usize>;
    async fn list(&self) -> anyhow::Result<Vec<Host>>;
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Host>>;
    async fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>>;
}
