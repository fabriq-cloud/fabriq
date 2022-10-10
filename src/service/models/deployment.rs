use fabriq_core::DeploymentMessage;

use crate::persistence::Persistable;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Deployment {
    pub id: String,
    pub name: String,
    pub workload_id: String,
    pub target_id: String,
    pub template_id: Option<String>,
    pub host_count: i32,
}

impl Persistable<Deployment> for Deployment {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Deployment> for DeploymentMessage {
    fn from(deployment: Deployment) -> Self {
        Self {
            id: deployment.id,
            name: deployment.name,
            workload_id: deployment.workload_id,
            target_id: deployment.target_id,
            template_id: deployment.template_id,
            host_count: deployment.host_count,
        }
    }
}

impl From<DeploymentMessage> for Deployment {
    fn from(deployment: DeploymentMessage) -> Self {
        Self {
            id: deployment.id,
            name: deployment.name,
            workload_id: deployment.workload_id,
            target_id: deployment.target_id,
            template_id: deployment.template_id,
            host_count: deployment.host_count,
        }
    }
}
