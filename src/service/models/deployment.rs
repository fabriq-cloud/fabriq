use akira_core::{DeploymentMessage, PersistableModel};
use diesel::sql_types::Integer;

use super::{Target, Workload};
use crate::schema::deployments;

#[derive(
    Associations, Clone, Debug, Eq, Identifiable, Insertable, PartialEq, Queryable, QueryableByName,
)]
#[table_name = "deployments"]
#[belongs_to(Workload)]
#[belongs_to(Target)]
pub struct Deployment {
    pub id: String,
    pub workload_id: String,
    pub target_id: String,

    #[sql_type = "Integer"]
    pub replicas: i32,
}

impl PersistableModel<Deployment, Deployment> for Deployment {
    fn new(new_deployment: Deployment) -> Deployment {
        return new_deployment;
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Deployment> for DeploymentMessage {
    fn from(deployment: Deployment) -> Self {
        return DeploymentMessage {
            id: deployment.id,
            workload_id: deployment.workload_id,
            target_id: deployment.target_id,
            replicas: deployment.replicas,
        };
    }
}
