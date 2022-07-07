use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;

use akira_core::common::TemplateIdRequest;
use akira_core::git::{GitRepo, RemoteGitRepo};

use akira_core::{
    get_current_or_previous_model, AssignmentMessage, ConfigMessage, ConfigTrait,
    DeploymentIdRequest, DeploymentMessage, DeploymentTrait, Event, EventType, HostMessage,
    ModelType, QueryConfigRequest, TargetMessage, TemplateMessage, TemplateTrait,
    WorkloadIdRequest, WorkloadMessage, WorkloadTrait, WorkspaceMessage,
};
use handlebars::Handlebars;
use std::fs;
use tonic::Request;

pub struct GitOpsProcessor {
    pub gitops_repo: Arc<dyn GitRepo>,
    pub private_ssh_key: String,

    pub config_client: Arc<dyn ConfigTrait>,
    pub deployment_client: Arc<dyn DeploymentTrait>,
    pub template_client: Arc<dyn TemplateTrait>,
    pub workload_client: Arc<dyn WorkloadTrait>,
}

impl Debug for GitOpsProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GitOpsProcessor")
    }
}

impl GitOpsProcessor {
    #[tracing::instrument(skip(self, event), fields(event_type = event.event_type))]
    pub async fn process(&mut self, event: &Event) -> anyhow::Result<()> {
        let model_type = event.model_type;

        match model_type {
            model_type if model_type == ModelType::Assignment as i32 => {
                self.process_assignment_event(event).await
            }
            model_type if model_type == ModelType::Deployment as i32 => {
                self.process_deployment_event(event).await
            }
            model_type if model_type == ModelType::Host as i32 => {
                self.process_host_event(event).await
            }
            model_type if model_type == ModelType::Target as i32 => {
                self.process_target_event(event).await
            }
            model_type if model_type == ModelType::Template as i32 => {
                self.process_template_event(event).await
            }
            model_type if model_type == ModelType::Workload as i32 => {
                self.process_workload_event(event).await
            }
            model_type if model_type == ModelType::Workspace as i32 => {
                self.process_workspace_event(event).await
            }
            _ => {
                panic!("unsupported model type: {:?}", event);
            }
        }
    }

    #[tracing::instrument]
    async fn render_assignment(
        &self,
        host_id: &str,
        workspace_id: &str,
        workload_id: &str,
        deployment_id: &str,
    ) -> anyhow::Result<()> {
        let assignment_path = GitOpsProcessor::make_assignment_path(
            host_id,
            workspace_id,
            workload_id,
            deployment_id,
        );

        let deployment_path =
            GitOpsProcessor::make_deployment_path(workspace_id, workload_id, deployment_id);

        let host_relative_deployment_path = format!("../../../../../{}", deployment_path);
        let template_string = fs::read_to_string("templates/assignment.yaml")?;

        let mut handlebars = Handlebars::new();
        handlebars.register_template_string("assignment", template_string)?;

        let key = "relative_deployment_path".to_owned();
        let mut values: HashMap<&str, &str> = HashMap::new();
        values.insert(&key, &host_relative_deployment_path);

        let rendered_assignment = handlebars.render("assignment", &values)?;

        self.gitops_repo
            .write_file(&assignment_path, rendered_assignment.as_bytes())?;
        self.gitops_repo.add_path(assignment_path.into())?;
        Ok(())
    }

    #[tracing::instrument]
    async fn process_assignment_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        let assignment = get_current_or_previous_model::<AssignmentMessage>(event)?;

        let deployment_request = Request::new(DeploymentIdRequest {
            deployment_id: assignment.deployment_id.clone(),
        });

        let deployment = self
            .deployment_client
            .get_by_id(deployment_request)
            .await?
            .into_inner();

        let workload_request = Request::new(WorkloadIdRequest {
            workload_id: deployment.workload_id.clone(),
        });

        let workload = self
            .workload_client
            .get_by_id(workload_request)
            .await?
            .into_inner();

        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                self.render_assignment(
                    &assignment.host_id,
                    &workload.workspace_id,
                    &deployment.workload_id,
                    &deployment.id,
                )
                .await?;
                tracing::info!("assignment id {} created", assignment.id);
            }
            event_type if event_type == EventType::Updated as i32 => {
                self.render_assignment(
                    &assignment.host_id,
                    &workload.workspace_id,
                    &deployment.workload_id,
                    &deployment.id,
                )
                .await?;
                tracing::info!("assignment id {} updated", assignment.id);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                let assignment_path = GitOpsProcessor::make_assignment_path(
                    &assignment.host_id,
                    &workload.workspace_id,
                    &deployment.workload_id,
                    &deployment.id,
                );

                self.gitops_repo.remove_dir(&assignment_path)?;
                tracing::info!("assignment id {} deleted", assignment.id);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        // TODO: Add generic capability to handle commit
        // TODO: Need to figure out how to plumb user effecting these changes here.
        self.gitops_repo.commit(
            "Tim Park",
            "timfpark@gmail.com",
            "Processed deployment event",
        )?;

        self.gitops_repo.push()
    }

    #[tracing::instrument]
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

    fn make_assignment_path(
        host_id: &str,
        workspace_id: &str,
        workload_id: &str,
        deployment_id: &str,
    ) -> String {
        format!(
            "hosts/{}/{}/{}/{}/kustomization.yaml",
            host_id, workspace_id, workload_id, deployment_id
        )
    }

    fn make_deployment_path(workspace_id: &str, workload_id: &str, deployment_id: &str) -> String {
        format!(
            "deployments/{}/{}/{}",
            workspace_id, workload_id, deployment_id
        )
    }

    fn make_template_path(template_id: &str, template_path: &str) -> String {
        format!("templates/{}/{}", template_id, template_path)
    }

    #[tracing::instrument]
    async fn render_deployment_template(
        &self,
        configs: &[ConfigMessage],
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
            let string_path = file_path.to_string_lossy();

            let mut values: HashMap<String, String> = HashMap::new();

            for config in configs {
                values.insert(config.key.clone(), config.value.clone());
            }

            values.insert("workspace".to_owned(), workload.workspace_id.clone());
            values.insert("workload".to_owned(), workload.id.clone());
            values.insert("deployment".to_owned(), deployment.id.clone());

            let rendered_template = handlebars.render(&template.id, &values)?;

            self.gitops_repo
                .write_file(&string_path, rendered_template.as_bytes())?;
            self.gitops_repo.add_path(file_path)?;
        }

        Ok(())
    }

    async fn render_deployment(
        &self,
        configs: &[ConfigMessage],
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

        self.render_deployment_template(configs, deployment, &template, workload)
            .await?;

        Ok(())
    }

    #[tracing::instrument]
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

        let config_request = Request::new(QueryConfigRequest {
            deployment_id: deployment.id.clone(),
            workload_id: deployment.workload_id.clone(),
        });

        let response = self.config_client.query(config_request).await?.into_inner();
        let configs = response.configs;

        let deployment_repo_path = Self::make_deployment_path(
            &workload.workspace_id,
            &deployment.workload_id,
            &deployment.id,
        );

        // Create / Update / Delete all remove current deployment from GitOps folder
        self.gitops_repo.remove_dir(&deployment_repo_path)?;

        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                self.render_deployment(&configs, &workload, &deployment)
                    .await?;
                tracing::info!("deployment id {} created", deployment.id);
            }
            event_type if event_type == EventType::Updated as i32 => {
                self.render_deployment(&configs, &workload, &deployment)
                    .await?;
                tracing::info!("deployment id {} updated", deployment.id);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                tracing::info!("deployment id {} deleted", deployment.id);
                // previous deployment directory removed above
                // so just commit deleted deployment below.
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        // TODO: Add generic capability to handle commit
        // TODO: Need to figure out how to plumb user effecting these changes here.
        self.gitops_repo.commit(
            "Tim Park",
            "timfpark@gmail.com",
            "Processed deployment event",
        )?;

        self.gitops_repo.push()
    }

    #[tracing::instrument]
    async fn process_host_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        let host = get_current_or_previous_model::<HostMessage>(event)?;

        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                tracing::info!("host id {} created (NOP)", host.id);
            }
            event_type if event_type == EventType::Updated as i32 => {
                tracing::info!("host id {} updated (NOP)", host.id);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                tracing::info!("host id {} deleted (NOP)", host.id);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        Ok(())
    }

    #[tracing::instrument]
    async fn process_target_event(&mut self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        let target = get_current_or_previous_model::<TargetMessage>(event)?;

        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                tracing::info!("target id {} created (NOP)", target.id);
            }
            event_type if event_type == EventType::Updated as i32 => {
                tracing::info!("target id {} updated (NOP)", target.id);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                tracing::info!("target id {} deleted (NOP)", target.id);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        Ok(())
    }

    #[tracing::instrument]
    async fn process_template_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        let template = get_current_or_previous_model::<TemplateMessage>(event)?;

        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                tracing::info!("template id {} created", template.id);
            }
            event_type if event_type == EventType::Updated as i32 => {
                tracing::info!("template id {} updated", template.id);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                tracing::info!("template id {} deleted", template.id);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        Ok(())
    }

    #[tracing::instrument]
    async fn process_workload_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        let workload = get_current_or_previous_model::<WorkloadMessage>(event)?;

        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                // NOP:

                // 1. Logically would just create a workload directory in GitOps repo.
                // 2. But this will happen with first deployment creation.
                tracing::info!("workload id {} created", workload.id);
            }
            event_type if event_type == EventType::Updated as i32 => {
                // TODO: Retrigger generation for all deployments in this workspace as the gitops path changed?
                tracing::info!("workload id {} updated", workload.id);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                // NOP:

                // 1. Logically would just delete a workload directory in GitOps repo.
                // 2. But this will happen with last deployment creation.

                tracing::info!("workload id {} deleted", workload.id);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        Ok(())
    }

    #[tracing::instrument]
    async fn process_workspace_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        let workspace = get_current_or_previous_model::<WorkspaceMessage>(event)?;

        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                // NOP:

                // 1. Logically would just create a workspace directory in GitOps repo.
                // 2. But this will happen with first deployment creation.
                tracing::info!("workspace id {} created", workspace.id);
            }
            event_type if event_type == EventType::Updated as i32 => {
                // TODO: Retrigger generation for all deployments in this workspace as the gitops path changed?
                tracing::info!("workspace id {} updated", workspace.id);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                // NOP:

                // 1. Logically would just delete a workspace directory in GitOps repo.
                // 2. But this will happen with last deployment creation.

                tracing::info!("workspace id {} deleted", workspace.id);
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
        AssignmentMessage, DeploymentMessage, EventType, ModelType, OperationId,
    };

    use std::{
        collections::hash_map::DefaultHasher,
        env, fs,
        hash::{Hash, Hasher},
        path::Path,
        sync::Arc,
    };

    use super::GitOpsProcessor;

    async fn create_processor_fixture(
        gitops_repo: Arc<dyn GitRepo>,
    ) -> anyhow::Result<GitOpsProcessor> {
        let private_ssh_key = env::var("PRIVATE_SSH_KEY").expect("PRIVATE_SSH_KEY must be set");

        let config_client = Arc::new(akira_core::api::mock::MockConfigClient {});
        let deployment_client = Arc::new(akira_core::api::mock::MockDeploymentClient {});
        let template_client = Arc::new(akira_core::api::mock::MockTemplateClient {});
        let workload_client = Arc::new(akira_core::api::mock::MockWorkloadClient {});

        Ok(GitOpsProcessor {
            gitops_repo,
            private_ssh_key,

            config_client,
            deployment_client,
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

        let deployment_pathbuf = format!("{}/deployment.yaml", deployment_path).into();
        let deployment_contents = gitops_repo.read_file(deployment_pathbuf).unwrap();

        assert!(!deployment_contents.is_empty());

        let mut hasher = DefaultHasher::new();
        deployment_contents.hash(&mut hasher);
        let deployment_hash = hasher.finish();

        assert_eq!(deployment_hash, 7141064955703910222);

        let _ = fs::remove_dir_all(deployment_path);

        deployment_event_impl(Arc::clone(&gitops_repo), EventType::Updated).await;

        let deployment_contents = gitops_repo
            .read_file(format!("{}/deployment.yaml", deployment_path).into())
            .unwrap();

        assert!(!deployment_contents.is_empty());

        let mut hasher = DefaultHasher::new();
        deployment_contents.hash(&mut hasher);
        let deployment_hash = hasher.finish();

        assert_eq!(deployment_hash, 7141064955703910222);

        deployment_event_impl(Arc::clone(&gitops_repo), EventType::Deleted).await;

        assert!(!Path::new(deployment_path).exists());
    }

    async fn assignment_event_impl(gitops_repo: Arc<MemoryGitRepo>, event_type: EventType) {
        let processor_gitops_repo: Arc<dyn GitRepo> = Arc::<MemoryGitRepo>::clone(&gitops_repo);
        let mut processor = create_processor_fixture(processor_gitops_repo)
            .await
            .unwrap();

        let assignment = AssignmentMessage {
            id: "deployment-fixture".to_owned(),
            host_id: "host-fixture".to_owned(),
            deployment_id: "deployment-fixture".to_owned(),
        };

        let operation_id = OperationId::create();

        let event = create_event(
            &None,
            &Some(assignment),
            event_type,
            ModelType::Assignment,
            &operation_id,
        );

        processor.process(&event).await.unwrap();
    }

    #[tokio::test]
    async fn test_process_assignment_events() {
        let assignment_path = GitOpsProcessor::make_assignment_path(
            "host-fixture",
            "workspace-fixture",
            "workload-fixture",
            "deployment-fixture",
        );

        let _ = fs::remove_dir_all(&assignment_path);

        let gitops_repo = Arc::new(MemoryGitRepo::new());

        assignment_event_impl(Arc::clone(&gitops_repo), EventType::Created).await;

        let assignment_contents = gitops_repo
            .read_file(assignment_path.clone().into())
            .unwrap();

        assert!(!assignment_contents.is_empty());

        let mut hasher = DefaultHasher::new();
        assignment_contents.hash(&mut hasher);
        let assignment_hash = hasher.finish();

        assert_eq!(assignment_hash, 15009592673730869112);

        let _ = fs::remove_dir_all(&assignment_path);

        assignment_event_impl(Arc::clone(&gitops_repo), EventType::Updated).await;

        let deployment_contents = gitops_repo
            .read_file(assignment_path.clone().into())
            .unwrap();

        assert!(!deployment_contents.is_empty());

        let mut hasher = DefaultHasher::new();
        assignment_contents.hash(&mut hasher);
        let assignment_hash = hasher.finish();

        assert_eq!(assignment_hash, 15009592673730869112);

        assignment_event_impl(Arc::clone(&gitops_repo), EventType::Deleted).await;

        assert!(!Path::new(&assignment_path).exists());
    }
}
