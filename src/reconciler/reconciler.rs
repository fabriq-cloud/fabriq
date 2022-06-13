// CoreProcessor reacts to events on models and creates / updates / or deletes other models in reaction.
// For example, if a deployment is created, it fetches its target, the hosts in the system, and creates assignments.
// It only handles work that is common across processors to keep the object model coherent.
// It does not handle the work that is specific to a specific workflow, for example truing up a GitOps repo.
// These processor specific workflows are implemented using external processors.

use std::sync::Arc;

use akira_core::{DeploymentMessage, Event, EventType, ModelType, Processor};
use prost::Message;

use crate::{
    models::{Assignment, Deployment},
    services::{AssignmentService, HostService, TargetService},
};

pub struct Reconciler {
    assignment_service: Arc<AssignmentService>,
    host_service: Arc<HostService>,
    target_service: Arc<TargetService>,
}

impl Reconciler {
    async fn process(&self, event: &Event) -> anyhow::Result<()> {
        println!("Processing event: {:?}", event);

        let model_type = event.model_type;

        match model_type {
            model_type if model_type == ModelType::Assignment as i32 => {
                // self.process_assignment_event(event).await
            }
            model_type if model_type == ModelType::Deployment as i32 => {
                self.process_deployment_event(event).await?;
            }
            model_type if model_type == ModelType::Host as i32 => {
                // self.process_host_event(event).await
            }
            model_type if model_type == ModelType::Target as i32 => {
                // self.process_target_event(event).await
            }
            model_type if model_type == ModelType::Template as i32 => {
                // self.process_template_event(event).await
            }
            model_type if model_type == ModelType::Workload as i32 => {
                // self.process_workload_event(event).await
            }
            model_type if model_type == ModelType::Workspace as i32 => {
                //self.process_workspace_event(event).await
            }
            _ => {
                panic!("unsupported model type: {:?}", event);
            }
        }

        Ok(())
    }
}
