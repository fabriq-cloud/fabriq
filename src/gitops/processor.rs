use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use akira_core::common::TemplateIdRequest;
use akira_core::git::{GitRepo, RemoteGitRepo};

use akira_core::{
    get_current_or_previous_model, DeploymentMessage, Event, EventType, ModelType, TemplateMessage,
    TemplateTrait, WorkloadIdRequest, WorkloadMessage, WorkloadTrait,
};
use handlebars::Handlebars;
use tonic::Request;

pub struct GitOpsProcessor {
    pub gitops_repo: Arc<dyn GitRepo>,
    pub private_ssh_key: String,

    pub template_client: Arc<dyn TemplateTrait>,
    pub workload_client: Arc<dyn WorkloadTrait>,
}

impl GitOpsProcessor {
    pub async fn process(&mut self, event: &Event) -> anyhow::Result<()> {
        let model_type = event.model_type;

        match model_type {
            model_type if model_type == ModelType::Assignment as i32 => {
                // create / update: link workload to host in host directory
                // delete: unlink workload from host in host directory
                self.process_assignment_event(event).await
            }
            model_type if model_type == ModelType::Deployment as i32 => {
                // render and commit deployment if created or updated
                // delete deployment directory if deleted
                self.process_deployment_event(event).await
            }
            model_type if model_type == ModelType::Host as i32 => {
                // delete: remove host directory
                self.process_host_event(event).await
            }
            model_type if model_type == ModelType::Target as i32 => {
                // NOP
                self.process_target_event(event).await
            }
            model_type if model_type == ModelType::Template as i32 => {
                // create/update: rerender all deployments or workloads using template
                // delete: NOP, shouldn't be any deployments using
                self.process_template_event(event).await
            }
            model_type if model_type == ModelType::Workload as i32 => {
                // create/update: rerender all deployments using workload
                // delete: remove whole workload directory from git repo
                self.process_workload_event(event).await
            }
            model_type if model_type == ModelType::Workspace as i32 => {
                // NOP
                self.process_workspace_event(event).await
            }
            _ => {
                panic!("unsupported model type: {:?}", event);
            }
        }
    }

    async fn process_assignment_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;

        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                tracing::info!("assignment created (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Updated as i32 => {
                tracing::info!("assignment updated (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                tracing::info!("assignment deleted (NOP): {:?}", event);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        Ok(())
    }

    async fn fetch_template_repo(
        &self,
        template: &TemplateMessage,
    ) -> anyhow::Result<RemoteGitRepo> {
        let template_path = format!("templates/{}", template.id);

        // TODO: Ability to use a different private ssh key for each template
        RemoteGitRepo::new(
            &template_path,
            &template.repository,
            &template.branch,
            &self.private_ssh_key,
        )
    }

    fn make_deployment_path(workspace_id: &str, workload_id: &str, deployment_id: &str) -> String {
        // deployments / workspace / workload / deployment
        format!(
            "deployments/{}/{}/{}",
            workspace_id, workload_id, deployment_id
        )
    }

    fn make_template_path(template_id: &str, template_path: &str) -> String {
        format!("templates/{}/{}", template_id, template_path)
    }

    async fn render_deployment_template(
        &self,
        deployment: &DeploymentMessage,
        template: &TemplateMessage,
        workload: &WorkloadMessage,
    ) -> anyhow::Result<()> {
        self.fetch_template_repo(template).await?;

        let template_repo_path = Self::make_template_path(&template.id, &template.path);
        let template_paths = fs::read_dir(template_repo_path)?;

        let deployment_repo_path = Self::make_deployment_path(
            &workload.workspace_id,
            &deployment.workload_id,
            &deployment.id,
        );

        for template_path in template_paths {
            let template_path = template_path?.path();
            let template_string = fs::read_to_string(template_path.clone())?;

            let mut handlebars = Handlebars::new();
            handlebars.register_template_string(&template.id, template_string)?;

            let file_name = template_path.file_name().unwrap();
            let file_path = Path::new(&deployment_repo_path).join(file_name);

            let values: HashMap<&str, &str> = HashMap::new();
            /*
            for config in configs {

                values.insert(config.key, config.value);
            }
            */

            let rendered_template = handlebars.render(&template.id, &values)?;

            self.gitops_repo
                .write_file(file_path, rendered_template.as_bytes())?;
        }

        Ok(())
    }

    async fn render_deployment(
        &self,
        workload: &WorkloadMessage,
        deployment: &DeploymentMessage,
    ) -> anyhow::Result<()> {
        let template_id = deployment
            .template_id
            .clone()
            .unwrap_or_else(|| workload.template_id.clone());

        let template_request = Request::new(TemplateIdRequest {
            template_id: template_id.clone(),
        });

        let template = self
            .template_client
            .get_by_id(template_request)
            .await?
            .into_inner();

        self.render_deployment_template(deployment, &template, workload)
            .await?;

        Ok(())
    }

    async fn process_deployment_event(&mut self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        let deployment = get_current_or_previous_model::<DeploymentMessage>(event)?;

        let workload_request = Request::new(WorkloadIdRequest {
            workload_id: deployment.workload_id.clone(),
        });

        let workload = self
            .workload_client
            .get_by_id(workload_request)
            .await?
            .into_inner();

        let deployment_repo_path = Self::make_deployment_path(
            &workload.workspace_id,
            &deployment.workload_id,
            &deployment.id,
        );

        // Create / Update / Delete all remove current deployment from GitOps folder
        self.gitops_repo.remove_dir(&deployment_repo_path)?;

        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                tracing::info!("deployment created {:?}", deployment);
                self.render_deployment(&workload, &deployment).await?;
            }
            event_type if event_type == EventType::Updated as i32 => {
                tracing::info!("deployment updated: {:?}", event);
                self.render_deployment(&workload, &deployment).await?;
            }
            event_type if event_type == EventType::Deleted as i32 => {
                tracing::info!("deployment deleted: {:?}", event);
                // just commit deleted deployment
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        // TODO: Need to figure out how to plumb user effecting these changes here.
        self.gitops_repo.commit(
            "Tim Park",
            "timfpark@gmail.com",
            "Processed deployment event",
        )?;

        self.gitops_repo.push()
    }

    async fn process_host_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                // Load targets, filter down to the ones that match the host
                // For each target, load all deployments
                // For each deployment, load all assignments
                // Run assignment check to see if we should add/delete an assignment for this host.
                // If so, create/delete assignment for this host.
                tracing::info!("host created (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Updated as i32 => {
                // Load targets, filter down to the ones that match the host
                // For each target, load all deployments
                // For each deployment, load all assignments
                // Run assignment check to see if we should add/delete an assignment for this host.
                // If so, create/delete assignment for this host.
                tracing::info!("host updated (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                // NOP: By definition a host can't be deleted until all of its assignments are removed.
                tracing::info!("host deleted (NOP): {:?}", event);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        Ok(())
    }

    async fn process_target_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                // NOP: By definition no deployment can be created without a target
                tracing::info!("target created (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Updated as i32 => {
                // Load all deployments that depend on this target.
                // For each, check assignments vs. target and adjust assignments as necessary.
                // Create / delete these assignments (linking the new hosts to the deployment will happen through Assignment Created event)
                tracing::info!("target updated (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                // NOP: By definition no deployment can still exist if this target could be deleted
                tracing::info!("target deleted (NOP): {:?}", event);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        Ok(())
    }

    async fn process_template_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                // NOP: By definition no deployment can be created without a template
                tracing::info!("template created (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Updated as i32 => {
                // 1. Query for all deployments that use this template
                // 2. Re-render deployment for all of these deployments.
                tracing::info!("template updated (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                // NOP: By definition no deployment can still exist if this target could be deleted
                tracing::info!("template deleted (NOP): {:?}", event);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        Ok(())
    }

    async fn process_workload_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                // NOP:

                // 1. Logically would just create a workload directory in GitOps repo.
                // 2. But this will happen with first deployment creation.
                tracing::info!("workload created (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Updated as i32 => {
                // TODO: Retrigger generation for all deployments in this workspace as the gitops path changed?
                tracing::info!("workload updated (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                // NOP:

                // 1. Logically would just delete a workload directory in GitOps repo.
                // 2. But this will happen with last deployment creation.

                tracing::info!("workload deleted (NOP): {:?}", event);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        Ok(())
    }

    async fn process_workspace_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                // NOP:

                // 1. Logically would just create a workspace directory in GitOps repo.
                // 2. But this will happen with first deployment creation.
                tracing::info!("workspace created (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Updated as i32 => {
                // TODO: Retrigger generation for all deployments in this workspace as the gitops path changed?
                tracing::info!("workspace updated (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                // NOP:

                // 1. Logically would just delete a workspace directory in GitOps repo.
                // 2. But this will happen with last deployment creation.

                tracing::info!("workspace deleted (NOP): {:?}", event);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use akira_core::{
        create_event,
        git::{GitRepo, MemoryGitRepo},
        DeploymentMessage, EventType, ModelType, OperationId,
    };

    use std::{env, fs, path::Path, sync::Arc};

    use super::GitOpsProcessor;

    async fn create_processor_fixture(
        gitops_repo: Arc<dyn GitRepo>,
    ) -> anyhow::Result<GitOpsProcessor> {
        let private_ssh_key = env::var("PRIVATE_SSH_KEY").expect("PRIVATE_SSH_KEY must be set");

        let template_client = Arc::new(akira_core::api::mock::MockTemplateClient {});
        let workload_client = Arc::new(akira_core::api::mock::MockWorkloadClient {});

        Ok(GitOpsProcessor {
            gitops_repo,
            private_ssh_key,

            template_client,
            workload_client,
        })
    }

    async fn deployment_event_impl(gitops_repo: Arc<MemoryGitRepo>, event_type: EventType) {
        let processor_gitops_repo: Arc<dyn GitRepo> = Arc::<MemoryGitRepo>::clone(&gitops_repo);
        let mut processor = create_processor_fixture(processor_gitops_repo)
            .await
            .unwrap();

        let deployment = DeploymentMessage {
            id: "deployment-fixture".to_owned(),
            target_id: "eastus2".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            template_id: Some("template-fixture".to_owned()),
            host_count: 2,
        };

        let operation_id = OperationId::create();

        let event = create_event(
            &None,
            &Some(deployment),
            event_type,
            ModelType::Deployment,
            &operation_id,
        );

        processor.process(&event).await.unwrap();
    }
    #[tokio::test]
    async fn test_process_deployment_events() {
        let deployment_path = "deployments/workspace-fixture/workload-fixture/deployment-fixture";
        let _ = fs::remove_dir_all(deployment_path);

        let gitops_repo = Arc::new(MemoryGitRepo::new());

        deployment_event_impl(Arc::clone(&gitops_repo), EventType::Created).await;

        let deployment_contents = gitops_repo
            .read_file(format!("{}/deployment.yaml", deployment_path).into())
            .unwrap();

        assert!(!deployment_contents.is_empty());

        let _ = fs::remove_dir_all(deployment_path);

        deployment_event_impl(Arc::clone(&gitops_repo), EventType::Updated).await;

        let deployment_contents = gitops_repo
            .read_file(format!("{}/deployment.yaml", deployment_path).into())
            .unwrap();

        assert!(!deployment_contents.is_empty());

        deployment_event_impl(Arc::clone(&gitops_repo), EventType::Deleted).await;

        assert!(!Path::new(deployment_path).exists());
    }
}
