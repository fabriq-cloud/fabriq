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
    use akira_core::test::{
        get_deployment_fixture, get_host_fixture, get_target_fixture, get_template_fixture,
        get_workload_fixture, get_workspace_fixture,
    };

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
    let workspace_fixture: Workspace = get_workspace_fixture(None).into();
    let workspace = workspace_persistence
        .get_by_id(&workspace_fixture.id)
        .unwrap();

    if workspace.is_none() {
        let workspace_fixture: Workspace = get_workspace_fixture(None).into();
        workspace_persistence.create(&workspace_fixture).unwrap();
    }

    let host_persistence = HostRelationalPersistence::default();
    let host_fixture: Host = get_host_fixture(None).into();
    let host_fixture = host_persistence.get_by_id(&host_fixture.id).unwrap();

    if host_fixture.is_none() {
        let host_fixture: Host = get_host_fixture(None).into();
        host_persistence.create(&host_fixture).unwrap();
    }

    let target_persistence = TargetRelationalPersistence::default();
    let target_fixture: Target = get_target_fixture(None).into();
    let target_fixture = target_persistence.get_by_id(&target_fixture.id).unwrap();

    if target_fixture.is_none() {
        let target_fixture: Target = get_target_fixture(None).into();
        target_persistence.create(&target_fixture).unwrap();
    }

    let template_persistence = TemplateRelationalPersistence::default();
    let template_fixture: Template = get_template_fixture(None).into();
    let template_fixture = template_persistence
        .get_by_id(&template_fixture.id)
        .unwrap();

    if template_fixture.is_none() {
        let template_fixture: Template = get_template_fixture(None).into();
        template_persistence.create(&template_fixture).unwrap();
    }

    let workload_persistence = WorkloadRelationalPersistence::default();
    let workload_fixture: Workload = get_workload_fixture(None).into();
    let workload = workload_persistence
        .get_by_id(&workload_fixture.id)
        .unwrap();

    if workload.is_none() {
        let workload_fixture: Workload = get_workload_fixture(None).into();
        workload_persistence.create(&workload_fixture).unwrap();
    }

    let deployment_persistence = DeploymentRelationalPersistence::default();
    let deployment_fixture: Deployment = get_deployment_fixture(None).into();
    let deployment = deployment_persistence
        .get_by_id(&deployment_fixture.id)
        .unwrap();

    if deployment.is_none() {
        let deployment_fixture: Deployment = get_deployment_fixture(None).into();
        deployment_persistence.create(&deployment_fixture).unwrap();
    }
}
