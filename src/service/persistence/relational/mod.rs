use std::sync::Mutex;

use lazy_static::lazy_static;

mod assignment;
mod config;
mod deployment;
mod host;
mod target;
mod template;
mod workload;
mod workspace;

pub use assignment::AssignmentRelationalPersistence;
pub use config::ConfigRelationalPersistence;
pub use deployment::DeploymentRelationalPersistence;
pub use host::HostRelationalPersistence;
pub use target::TargetRelationalPersistence;
pub use template::TemplateRelationalPersistence;
pub use workload::WorkloadRelationalPersistence;
pub use workspace::WorkspaceRelationalPersistence;

lazy_static! {
    pub static ref FIXTURES_CREATED: Mutex<bool> = Mutex::new(false);
}

#[cfg(test)]
pub fn ensure_fixtures() {
    use crate::{
        models::{Deployment, Host, Target, Template, Workload, Workspace},
        persistence::Persistence,
    };

    // tests run multithreaded, so we need to ensure that we block all but the first
    let mut fixtures_created = FIXTURES_CREATED.lock().unwrap();

    if *fixtures_created {
        // fixtures already created
        return;
    } else {
        *fixtures_created = true;
    }

    let workspace_persistence = WorkspaceRelationalPersistence::default();
    let workspace_fixture = workspace_persistence
        .get_by_id("workspace-fixture")
        .unwrap();

    if workspace_fixture.is_none() {
        let workspace_fixture = Workspace {
            id: "workspace-fixture".to_string(),
        };
        workspace_persistence.create(&workspace_fixture).unwrap();
    }

    let host_persistence = HostRelationalPersistence::default();
    let host_fixture = host_persistence.get_by_id("host-fixture").unwrap();

    if host_fixture.is_none() {
        let host_fixture = Host {
            id: "host-fixture".to_string(),
            labels: vec!["region:eastus2".to_string()],
        };
        host_persistence.create(&host_fixture).unwrap();
    }

    let target_persistence = TargetRelationalPersistence::default();
    let target_fixture = target_persistence.get_by_id("target-fixture").unwrap();

    if target_fixture.is_none() {
        let target_fixture = Target {
            id: "target-fixture".to_string(),
            labels: vec!["location:eastus2".to_string()],
        };
        target_persistence.create(&target_fixture).unwrap();
    }

    let template_persistence = TemplateRelationalPersistence::default();
    let template_fixture = template_persistence.get_by_id("template-fixture").unwrap();

    if template_fixture.is_none() {
        let template_fixture = Template {
            id: "template-fixture".to_string(),
            repository: "https://github.com/timfpark/deployment-templates".to_string(),
            branch: "main".to_string(),
            path: "./test-template".to_string(),
        };
        template_persistence.create(&template_fixture).unwrap();
    }

    let workload_persistence = WorkloadRelationalPersistence::default();
    let workload_fixture = workload_persistence.get_by_id("workload-fixture").unwrap();

    if workload_fixture.is_none() {
        let workload_fixture = Workload {
            id: "workload-fixture".to_string(),
            workspace_id: "workspace-fixture".to_string(),
            template_id: "template-fixture".to_string(),
        };
        workload_persistence.create(&workload_fixture).unwrap();
    }

    let deployment_persistence = DeploymentRelationalPersistence::default();
    let deployment_fixture = deployment_persistence
        .get_by_id("deployment-fixture")
        .unwrap();

    if deployment_fixture.is_none() {
        let deployment_fixture = Deployment {
            id: "deployment-fixture".to_string(),
            workload_id: "workload-fixture".to_string(),
            target_id: "target-fixture".to_string(),
            template_id: Some("template-fixture".to_string()),
            host_count: 2,
        };
        deployment_persistence.create(&deployment_fixture).unwrap();
    }
}
