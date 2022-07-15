use akira_core::WorkloadMessage;

use crate::models::Template;
use crate::persistence::PersistableModel;
use crate::schema::workloads;

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
#[table_name = "workloads"]
#[belongs_to(Template)]
pub struct Workload {
    pub id: String,
    pub name: String,
    pub team_id: String,
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
            name: workload.name,
            team_id: workload.team_id,
            template_id: workload.template_id,
        }
    }
}

impl From<WorkloadMessage> for Workload {
    fn from(workload: WorkloadMessage) -> Self {
        Self {
            id: workload.id,
            name: workload.name,
            team_id: workload.team_id,
            template_id: workload.template_id,
        }
    }
}
