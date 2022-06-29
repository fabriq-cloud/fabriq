use std::collections::HashMap;
use std::fs;
use std::path::Path;

use akira_core::common::TemplateIdRequest;
use akira_core::template::template_client::TemplateClient;
use akira_core::workload::workload_client::WorkloadClient;
use akira_core::{
    DeploymentMessage, Event, EventType, ModelType, TemplateMessage, WorkloadIdRequest,
    WorkloadMessage,
};
use handlebars::Handlebars;
use prost::Message;
use tonic::codegen::InterceptedService;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::service::Interceptor;
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;
use crate::repo::GitRepo;

pub struct GitOpsProcessor {
    gitops_repo: GitRepo,

    channel: Channel,
    token: MetadataValue<Ascii>,
}

impl GitOpsProcessor {
    pub async fn new(gitops_repo: GitRepo) -> anyhow::Result<Self> {
        let context = Context::default();
        let channel = Channel::from_static(context.endpoint).connect().await?;
        let token: MetadataValue<Ascii> = context.token.parse()?;

        Ok(Self {
            gitops_repo,

            channel,
            token,
        })
    }

    fn create_template_client(
        &self,
    ) -> TemplateClient<InterceptedService<Channel, impl Interceptor + '_>> {
        TemplateClient::with_interceptor(self.channel.clone(), move |mut req: Request<()>| {
            req.metadata_mut()
                .insert("authorization", self.token.clone());
            Ok(req)
        })
    }

    fn create_workload_client(
        &self,
    ) -> WorkloadClient<InterceptedService<Channel, impl Interceptor + '_>> {
        WorkloadClient::with_interceptor(self.channel.clone(), move |mut req: Request<()>| {
            req.metadata_mut()
                .insert("authorization", self.token.clone());
            Ok(req)
        })
    }

    fn get_current_or_previous_model<ModelMessage: Default + Message>(
        event: &Event,
    ) -> anyhow::Result<ModelMessage> {
        let message: ModelMessage =
            if let Some(serialized_current_model) = &event.serialized_current_model {
                ModelMessage::decode(&**serialized_current_model)?
            } else if let Some(serialized_previous_model) = &event.serialized_previous_model {
                ModelMessage::decode(&**serialized_previous_model)?
            } else {
                return Err(anyhow::anyhow!(
                    "Event received without previous or current model {:?}",
                    event
                ));
            };

        Ok(message)
    }

    pub async fn process(&mut self, event: &Event) -> anyhow::Result<()> {
        println!("Processing event: {:?}", event);

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

    async fn fetch_template_repo(&self, template: &TemplateMessage) -> anyhow::Result<GitRepo> {
        // TODO: Do we want to use the same private key for all deployments?
        let template_path = format!("templates/{}", template.id);

        GitRepo::new(
            &template_path,
            &template.repository,
            &template.branch,
            &self.gitops_repo.private_ssh_key,
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
                .write_text_file(file_path, &rendered_template)
                .await?;
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

        let template_request = tonic::Request::new(TemplateIdRequest {
            template_id: template_id.clone(),
        });

        let template = self
            .create_template_client()
            .get_by_id(template_request)
            .await?
            .into_inner();

        self.render_deployment_template(deployment, &template, workload)
            .await?;

        Ok(())
    }

    async fn process_deployment_event(&mut self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        let deployment = Self::get_current_or_previous_model::<DeploymentMessage>(event)?;

        let workload_request = Request::new(WorkloadIdRequest {
            workload_id: deployment.workload_id.clone(),
        });

        let workload = self
            .create_workload_client()
            .get_by_id(workload_request)
            .await?
            .into_inner();

        let deployment_repo_path = Self::make_deployment_path(
            &workload.workspace_id,
            &deployment.workload_id,
            &deployment.id,
        );

        // Create / Update / Delete all remove current deployment from GitOps folder
        self.gitops_repo.remove_dir(&deployment_repo_path).await?;

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
        self.gitops_repo
            .commit(
                "Tim Park",
                "timfpark@gmail.com",
                "Processed deployment event",
            )
            .await?;

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
    use std::{env, fs};

    use crate::repo::GitRepo;

    use super::GitOpsProcessor;

    async fn _create_processor_fixture() -> GitOpsProcessor {
        let branch = "main";
        let local_path = "fixtures/gitops-processor-test";
        let private_ssh_key = env::var("PRIVATE_SSH_KEY").expect("PRIVATE_SSH_KEY must be set");
        let repo_url = "git@github.com:timfpark/akira-clone-repo-test.git";

        // if this fails, it just means the repo hasn't been created yet
        let _ = fs::remove_dir_all(local_path);
        fs::create_dir_all(local_path).unwrap();

        let gitops_repo = GitRepo::new(local_path, repo_url, branch, &private_ssh_key).unwrap();
        GitOpsProcessor::new(gitops_repo).await.unwrap()
    }

    /*
    use akira_core::{create_event, DeploymentMessage, EventType, ModelType, OperationId};
    #[tokio::test]
    async fn test_process_deployment_create_event() {
        let _processor = create_processor_fixture().await;

        let deployment = DeploymentMessage {
            id: "deployment-fixture".to_owned(),
            target_id: "eastus2".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            template_id: Some("template-fixture".to_owned()),
            host_count: 2,
        };

        let operation_id = OperationId::create();

        let _event = create_event(
            &None,
            &Some(deployment),
            EventType::Created,
            ModelType::Deployment,
            &operation_id,
        );

        // processor.process(&event).await.unwrap();

        // check filesystem to make sure deployment was created
    }
    */

    #[test]
    fn test_process_deployment_update_event() {}

    #[test]
    fn test_process_deployment_delete_event() {}
}
