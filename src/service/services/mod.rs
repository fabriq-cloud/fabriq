mod assignment;
mod deployment;
mod host;
mod target;
mod template;
mod workload;
mod workspace;

pub use assignment::AssignmentService;
use async_trait::async_trait;
pub use deployment::DeploymentService;
pub use host::HostService;
pub use target::TargetService;
pub use template::TemplateService;
pub use workload::WorkloadService;
pub use workspace::WorkspaceService;

#[async_trait]
pub trait AssignmentPersistence<Assignment, NewAssignment>: Send + Sync {
    async fn create(&self, new_model: NewAssignment) -> anyhow::Result<String>;
    async fn delete(&self, model_id: &str) -> anyhow::Result<usize>;
    async fn list(&self) -> anyhow::Result<Vec<Assignment>>;
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Assignment>>;
    async fn get_by_deployment_id(&self, id: &str) -> anyhow::Result<Vec<Assignment>>;
}
