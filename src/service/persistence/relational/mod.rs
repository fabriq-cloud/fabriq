mod assignment;
mod deployment;
mod host;
mod target;
mod template;
mod workload;
mod workspace;

pub use assignment::AssignmentRelationalPersistence;
pub use deployment::DeploymentRelationalPersistence;
pub use host::HostRelationalPersistence;
pub use target::TargetRelationalPersistence;
pub use template::TemplateRelationalPersistence;
pub use workload::WorkloadRelationalPersistence;
pub use workspace::WorkspaceRelationalPersistence;

#[cfg(test)]

pub fn ensure_fixtures() {
    use crate::{
        models::{Deployment, Host, Target, Template, Workload, Workspace},
        persistence::Persistence,
    };

    let deployment_fixture = Deployment {
        id: "deployment-fixture".to_string(),
        workload_id: "workload-fixture".to_string(),
        target_id: "target-fixture".to_string(),
        hosts: 2,
    };

    let deployment_persistence = DeploymentRelationalPersistence::default();

    let _ = deployment_persistence.create(&deployment_fixture);

    let host_fixture = Host {
        id: "host-fixture".to_string(),
        labels: vec!["region:eastus2".to_string()],
    };

    let host_persistence = HostRelationalPersistence::default();

    let _ = host_persistence.create(&host_fixture);

    let target_fixture = Target {
        id: "target-fixture".to_string(),
        labels: vec!["location:eastus2".to_string()],
    };

    let target_persistence = TargetRelationalPersistence::default();

    let _ = target_persistence.create(&target_fixture);

    let template_fixture = Template {
        id: "template-fixture".to_string(),
        repository: "https://github.com/timfpark/deployment-templates".to_string(),
        branch: "main".to_string(),
        path: "./test-template".to_string(),
    };

    let template_persistence = TemplateRelationalPersistence::default();
    let _ = template_persistence.create(&template_fixture);

    let workload_fixture = Workload {
        id: "workload-fixture".to_string(),
        workspace_id: "workspace-fixture".to_string(),
        template_id: "template-fixture".to_string(),
    };

    let workload_persistence = WorkloadRelationalPersistence::default();
    let _ = workload_persistence.create(&workload_fixture);

    let workspace_fixture = Workspace {
        id: "workspace-fixture".to_string(),
    };

    let workspace_persistence = WorkspaceRelationalPersistence::default();
    let _ = workspace_persistence.create(&workspace_fixture);
}
