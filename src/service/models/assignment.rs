use fabriq_core::AssignmentMessage;

use crate::persistence::Persistable;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Assignment {
    pub id: String,

    pub deployment_id: String,
    pub host_id: String,
}

impl Persistable<Assignment> for Assignment {
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
