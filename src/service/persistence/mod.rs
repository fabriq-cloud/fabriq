use akira_core::Persistence;

use crate::models::{Assignment, Host, Target};

pub mod memory;
pub mod relational;

pub trait AssignmentPersistence: Send + Sync + Persistence<Assignment> {
    fn get_by_deployment_id(&self, id: &str) -> anyhow::Result<Vec<Assignment>>;
}

pub trait HostPersistence: Send + Sync + Persistence<Host> {
    fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>>;
}
