use crate::persistence::Persistable;
use akira_core::WorkloadMessage;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Workload {
    pub id: String,
    pub name: String,
    pub team_id: String,
    pub template_id: String,
}

impl Persistable<Workload> for Workload {
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
