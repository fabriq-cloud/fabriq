use async_trait::async_trait;

use crate::models::Assignment;

pub mod memory;
pub mod relational;

#[async_trait]
pub trait AssignmentPersistence: Send + Sync {
    async fn create(&self, new_model: Assignment) -> anyhow::Result<String>;
    async fn delete(&self, model_id: &str) -> anyhow::Result<usize>;
    async fn list(&self) -> anyhow::Result<Vec<Assignment>>;
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Assignment>>;
    async fn get_by_deployment_id(&self, id: &str) -> anyhow::Result<Vec<Assignment>>;
}
