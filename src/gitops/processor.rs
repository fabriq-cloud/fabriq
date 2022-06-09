use akira_core::{Event, EventType, ModelType, Processor};
use async_trait::async_trait;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::transport::Channel;

use crate::context::Context;

pub struct GitOpsProcessor {
    _channel: Channel,
    _token: MetadataValue<Ascii>,
}

#[async_trait]
impl Processor for GitOpsProcessor {
    async fn process(&self, event: &Event) -> anyhow::Result<()> {
        println!("Processing event: {:?}", event);

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
}

impl GitOpsProcessor {
    pub async fn new() -> anyhow::Result<Self> {
        let context = Context::default();
        let channel = Channel::from_static(context.endpoint).connect().await?;
        let token: MetadataValue<Ascii> = context.token.parse()?;

        Ok(Self {
            _channel: channel,
            _token: token,
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

    async fn process_deployment_event(&self, event: &Event) -> anyhow::Result<()> {
        let event_type = event.event_type;
        match event_type {
            event_type if event_type == EventType::Created as i32 => {
                /*
                let _ = match self.target_service.get_by_id(&deployment.target_id).await? {
                    None => {
                        return Err(anyhow::anyhow!("target {} not found", deployment.target_id))
                    }
                    Some(target) => target,
                };

                let assignments = self
                    .workspace_service
                    .get_by_deployment_id(&deployment.id)
                    .await?;

                for assignment in assignments {
                    let host = self.host
                }

                */

                // check current assignments and make sure still valid
                // if not valid, delete them
                // if valid, add them to valid list

                // if valid list is short versus number of hosts requested, add

                // Check assignments vs. target and adjust assignments as necessary.
                // Create / delete these assignments (linking the host to the deployment will happen through Assignment Created event)
                // Render deployment and commit in GitOps workloads folder
                tracing::info!("deployment created (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Updated as i32 => {
                // Render deployment and commit in GitOps folder
                tracing::info!("deployment updated (NOP): {:?}", event);
            }
            event_type if event_type == EventType::Deleted as i32 => {
                // Delete assignments for deployment (Assignment Deleted event will handle unlinking host from deployment)
                // Remove deployment from GitOps folder
                tracing::info!("deployment deleted (NOP): {:?}", event);
            }
            _ => {
                panic!("unsupported event type: {:?}", event);
            }
        }

        Ok(())
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
