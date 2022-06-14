use akira_core::WorkloadMessage;

use crate::models::{Template, Workspace};
use crate::persistence::PersistableModel;
use crate::schema::workloads;

#[derive(
    Associations, Clone, Debug, Eq, Identifiable, Insertable, PartialEq, Queryable, QueryableByName,
)]
#[table_name = "workloads"]
#[belongs_to(Template)]
#[belongs_to(Workspace)]
pub struct Workload {
    pub id: String,

    pub workspace_id: String,
    pub template_id: String,
}

impl PersistableModel<Workload> for Workload {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Workload> for WorkloadMessage {
    fn from(workload: Workload) -> Self {
        Self {
            id: workload.id,
            workspace_id: workload.workspace_id,
            template_id: workload.template_id,
        }
    }
}
