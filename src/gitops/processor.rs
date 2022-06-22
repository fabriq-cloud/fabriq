use akira::models::Deployment;
use akira_core::assignment::assignment_client::AssignmentClient;
use akira_core::host::host_client::HostClient;
use akira_core::target::target_client::TargetClient;
use akira_core::workload::workload_client::WorkloadClient;
use akira_core::{DeploymentMessage, Event, EventType, ModelType, WorkloadIdRequest};
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

impl<'a> GitOpsProcessor {
    pub async fn new(gitops_repo: GitRepo) -> anyhow::Result<Self> {
        let context = Context::default();
        let channel = Channel::from_static(context.endpoint).connect().await?;
        let token: MetadataValue<Ascii> = context.token.parse()?;

        Ok(Self {
            channel,
            gitops_repo,
            token,
        })
    }

    fn get_current_or_previous_model<ModelMessage: Default + Message, Model: From<ModelMessage>>(
        event: &Event,
    ) -> anyhow::Result<Model> {
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

        Ok(message.into())
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

    pub fn create_assignment_client(
        &self,
    ) -> AssignmentClient<InterceptedService<Channel, impl Interceptor + '_>> {
        AssignmentClient::with_interceptor(self.channel.clone(), move |mut req: Request<()>| {
            req.metadata_mut()
                .insert("authorization", self.token.clone());
            Ok(req)
        })
    }

    pub fn create_host_client(
        &self,
    ) -> HostClient<InterceptedService<Channel, impl Interceptor + '_>> {
        HostClient::with_interceptor(self.channel.clone(), move |mut req: Request<()>| {
            req.metadata_mut()
                .insert("authorization", self.token.clone());
            Ok(req)
        })
    }

    pub fn create_target_client(
        &self,
    ) -> TargetClient<InterceptedService<Channel, impl Interceptor + '_>> {
        TargetClient::with_interceptor(self.channel.clone(), move |mut req: Request<()>| {
            req.metadata_mut()
                .insert("authorization", self.token.clone());
            Ok(req)
        })
    }

    pub fn create_workload_client(
        &self,
    ) -> WorkloadClient<InterceptedService<Channel, impl Interceptor + '_>> {
        WorkloadClient::with_interceptor(self.channel.clone(), move |mut req: Request<()>| {
            req.metadata_mut()
                .insert("authorization", self.token.clone());
            Ok(req)
        })
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

    async fn process_deployment_event(&mut self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        let deployment =
            Self::get_current_or_previous_model::<DeploymentMessage, Deployment>(event)?;

        let mut workload_client = self.create_workload_client();

        let workload_request = Request::new(WorkloadIdRequest {
            workload_id: deployment.workload_id.clone(),
        });

        let workload = workload_client
            .get_by_id(workload_request)
            .await?
            .into_inner();

        // deployments / workspace / workload / deployment
        let deployment_repo_path = format!(
            "deployments/{}/{}/{}",
            workload.workspace_id, deployment.workload_id, deployment.id
        );

        self.gitops_repo.add(&deployment_repo_path).await?;

        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                // Delete current deployment

                // Render deployment into
                // Commit in GitOps folder
                let _assignment_client = self.create_assignment_client();
                let _host_client = self.create_host_client();
                let _target_client = self.create_target_client();
            }
            event_type if event_type == EventType::Updated as i32 => {
                // Render deployment and commit in GitOps folder
                tracing::info!("deployment updated (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                // Remove deployment from GitOps folder
                self.gitops_repo.remove_dir(&deployment_repo_path).await?;
                tracing::info!("deployment deleted (NOP): {:?}", event);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

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
