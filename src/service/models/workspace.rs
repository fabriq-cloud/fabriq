use akira_core::WorkspaceMessage;

use crate::{persistence::PersistableModel, schema::workspaces};

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
#[table_name = "workspaces"]
pub struct Workspace {
    pub id: String, // cribbage-team
}

impl PersistableModel<Workspace> for Workspace {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Workspace> for WorkspaceMessage {
    fn from(workload: Workspace) -> Self {
        Self { id: workload.id }
    }
}

impl From<WorkspaceMessage> for Workspace {
    fn from(workload: WorkspaceMessage) -> Self {
        Self { id: workload.id }
    }
}
