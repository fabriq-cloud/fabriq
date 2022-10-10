use akira_core::HostMessage;

use crate::persistence::Persistable;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Host {
    pub id: String,
    pub labels: Vec<String>,
}

impl Persistable<Host> for Host {
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
