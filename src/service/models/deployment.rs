use akira_core::DeploymentMessage;
use diesel::sql_types::Integer;

use super::{Target, Workload};
use crate::{persistence::PersistableModel, schema::deployments};

#[derive(
    Associations,
    Clone,
    Debug,
    Default,
    Eq,
    Identifiable,
    Insertable,
    PartialEq,
    Queryable,
    QueryableByName,
)]
#[table_name = "deployments"]
#[belongs_to(Workload)]
#[belongs_to(Target)]
pub struct Deployment {
    pub id: String,
    pub workload_id: String,
    pub target_id: String,
    pub template_id: Option<String>,

    #[sql_type = "Integer"]
    pub host_count: i32,
    pub name: String,
}

impl PersistableModel<Deployment> for Deployment {
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
