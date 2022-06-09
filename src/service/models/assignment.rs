use akira_core::{AssignmentMessage, PersistableModel};

use crate::models::{Deployment, Host};
use crate::schema::assignments;

#[derive(
    Associations, Clone, Debug, Eq, Identifiable, Insertable, PartialEq, Queryable, QueryableByName,
)]
#[table_name = "assignments"]
#[belongs_to(Deployment)]
#[belongs_to(Host)]
pub struct Assignment {
    pub id: String,

    pub deployment_id: String,
    pub host_id: String,
}

impl PersistableModel<Assignment, Assignment> for Assignment {
    fn new(new_assignment: Assignment) -> Assignment {
        return new_assignment;
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Assignment> for AssignmentMessage {
    fn from(assignment: Assignment) -> Self {
        return AssignmentMessage {
            id: assignment.id,
            deployment_id: assignment.deployment_id,
            host_id: assignment.host_id,
        };
    }
}
