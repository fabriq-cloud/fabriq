use akira_core::{PersistableModel, TargetMessage};

use crate::schema::targets;

#[derive(Clone, Debug, Insertable, Eq, PartialEq, Queryable, QueryableByName)]
#[table_name = "targets"]
pub struct Target {
    pub id: String,

    pub labels: Vec<String>,
}

impl PersistableModel<Target> for Target {
    fn new(new_target: Target) -> Self {
        new_target
    }

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
