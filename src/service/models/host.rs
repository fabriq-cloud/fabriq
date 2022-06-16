use akira_core::HostMessage;

use crate::{persistence::PersistableModel, schema::hosts};

#[derive(Clone, Debug, Default, Insertable, Eq, PartialEq, Queryable, QueryableByName)]
#[table_name = "hosts"]
pub struct Host {
    pub id: String,
    pub labels: Vec<String>,
}

impl PersistableModel<Host> for Host {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Host> for HostMessage {
    fn from(host: Host) -> Self {
        Self {
            id: host.id,
            labels: host.labels,
        }
    }
}

impl From<HostMessage> for Host {
    fn from(host_message: HostMessage) -> Self {
        Self {
            id: host_message.id,
            labels: host_message.labels,
        }
    }
}
