use akira_core::TargetMessage;

use crate::persistence::Persistable;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Target {
    pub id: String,

    pub labels: Vec<String>,
}

impl Persistable<Target> for Target {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Target> for TargetMessage {
    fn from(target: Target) -> Self {
        Self {
            id: target.id,
            labels: target.labels,
        }
    }
}

impl From<TargetMessage> for Target {
    fn from(target: TargetMessage) -> Self {
        Self {
            id: target.id,
            labels: target.labels,
        }
    }
}
