use akira_core::{HostMessage, PersistableModel};
use diesel::sql_types::{BigInt, Integer};

use crate::schema::hosts;

#[derive(Clone, Debug, Insertable, Eq, PartialEq, Queryable, QueryableByName)]
#[table_name = "hosts"]
pub struct Host {
    pub id: String,
    pub labels: Vec<String>,

    #[sql_type = "Integer"]
    pub cpu_capacity: i32, // in millicores

    #[sql_type = "BigInt"]
    pub memory_capacity: i64, // in KB
}

impl PersistableModel<Host, Host> for Host {
    fn new(new_host: Host) -> Host {
        new_host
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Host> for HostMessage {
    fn from(host: Host) -> Self {
        Self {
            id: host.id,
            labels: host.labels.clone(),
            cpu_capacity: host.cpu_capacity,
            memory_capacity: host.memory_capacity,
        }
    }
}
