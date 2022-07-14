use akira_core::AssignmentMessage;

use crate::models::{Deployment, Host};
use crate::persistence::PersistableModel;
use crate::schema::assignments;

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
#[table_name = "assignments"]
#[belongs_to(Deployment)]
#[belongs_to(Host)]
pub struct Assignment {
    pub id: String,

    pub deployment_id: String,
    pub host_id: String,
}

impl PersistableModel<Assignment> for Assignment {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Assignment> for AssignmentMessage {
    fn from(assignment: Assignment) -> Self {
        Self {
            id: assignment.id,
            deployment_id: assignment.deployment_id,
            host_id: assignment.host_id,
        }
    }
}

impl From<AssignmentMessage> for Assignment {
    fn from(assignment: AssignmentMessage) -> Self {
        Self {
            id: assignment.id,
            deployment_id: assignment.deployment_id,
            host_id: assignment.host_id,
        }
    }
}
