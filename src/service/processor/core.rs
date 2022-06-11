// CoreProcessor reacts to events on models and creates / updates / or deletes other models in reaction.
// For example, if a deployment is created, it fetches its target, the hosts in the system, and creates assignments.
// It only handles work that is common across processors to keep the object model coherent.
// It does not handle the work that is specific to a specific workflow, for example truing up a GitOps repo.
// These processor specific workflows are implemented using external processors.

use std::sync::Arc;

use akira_core::{DeploymentMessage, Event, EventType, ModelType, Processor};
use async_trait::async_trait;
use prost::Message;

use crate::{
    models::{Assignment, Deployment},
    services::{AssignmentService, HostService, TargetService},
};

pub struct CoreProcessor {
    assignment_service: AssignmentService,
    host_service: Arc<HostService>,
    target_service: Arc<TargetService>,
}

#[async_trait]
impl Processor for CoreProcessor {
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

impl CoreProcessor {
    async fn process_deployment_event(&self, event: &Event) -> anyhow::Result<()> {
        let deployment: Deployment = DeploymentMessage::decode(&*event.serialized_model)?.into();

        let desired_hosts: usize = if event.event_type == EventType::Deleted as i32 {
            0
        } else {
            deployment.hosts as usize
        };

        let target = self.target_service.get_by_id(&deployment.target_id).await?;
        let target = match target {
            Some(target) => target,
            None => {
                return Err(anyhow::anyhow!(
                    "couldn't find target with id {}",
                    deployment.target_id
                ))
            }
        };

        let mut available_hosts = self.host_service.get_matching_target(&target).await?;

        let mut current_assignments = self
            .assignment_service
            .get_by_deployment_id(&deployment.id)
            .await?;

        let mut assignments_to_create = Vec::new();
        let mut assignments_to_delete = Vec::new();

        for assignment in current_assignments.iter() {
            if available_hosts
                .iter()
                .filter(|host| host.id == assignment.host_id)
                .count()
                == 0
            {
                assignments_to_delete.push(assignment.id.clone());
            } else {
                // host still matches target, so remove it from matching_hosts so we don't double assign it.
                available_hosts.retain(|host| host.id == assignment.host_id);
            }
        }

        if current_assignments.len() > desired_hosts {
            let delete_count = current_assignments.len() - desired_hosts;

            let delete_assignment_ids: Vec<String> = current_assignments
                .drain(0..delete_count)
                .map(|assignment| assignment.id)
                .collect();

            assignments_to_delete.extend(delete_assignment_ids);
        } else {
            let create_count = desired_hosts - current_assignments.len();

            assignments_to_create = available_hosts
                .drain(0..create_count)
                .map(|host| Assignment {
                    id: Assignment::make_id(&deployment.id, &host.id),
                    deployment_id: deployment.id.clone(),
                    host_id: host.id.clone(),
                })
                .collect();
        }

        for assignment in assignments_to_create {
            self.assignment_service
                .create(&assignment, &event.operation_id)
                .await?;
        }

        for assignment in assignments_to_delete {
            self.assignment_service
                .delete(&assignment, &event.operation_id)
                .await?;
        }

        Ok(())
    }
}
