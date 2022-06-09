use akira_core::{PersistableModel, WorkspaceMessage};

use crate::schema::workspaces;

#[derive(
    Associations, Clone, Debug, Eq, Identifiable, Insertable, PartialEq, Queryable, QueryableByName,
)]
#[table_name = "workspaces"]
pub struct Workspace {
    pub id: String, // cribbage-team
}

impl PersistableModel<Workspace, Workspace> for Workspace {
    fn new(new_workspace: Workspace) -> Workspace {
        return new_workspace;
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Workspace> for WorkspaceMessage {
    fn from(workload: Workspace) -> Self {
        return WorkspaceMessage { id: workload.id };
    }
}
