// CoreProcessor reacts to events on models and creates / updates / or deletes other models in reaction.
// For example, if a deployment is created, it fetches its target, the hosts in the system, and creates assignments.
// It only handles work that is common across processors to keep the object model coherent.
// It does not handle the work that is specific to a specific workflow, for example truing up a GitOps repo.
// These processor specific workflows are implemented using external processors.

use std::sync::Arc;

use akira_core::{DeploymentMessage, Event, EventType, ModelType};
use prost::Message;

use akira::{
    models::{Assignment, Deployment, Host},
    services::{
        AssignmentService, DeploymentService, HostService, TargetService, TemplateService,
        WorkloadService, WorkspaceService,
    },
};

pub struct Reconciler {
    pub assignment_service: Arc<AssignmentService>,
    pub deployment_service: Arc<DeploymentService>,
    pub host_service: Arc<HostService>,
    pub target_service: Arc<TargetService>,
    pub template_service: Arc<TemplateService>,
    pub workload_service: Arc<WorkloadService>,
    pub workspace_service: Arc<WorkspaceService>,
}

impl Reconciler {
    pub fn process(&self, event: &Event) -> anyhow::Result<()> {
        println!("Processing event: {:?}", event);

        let model_type = event.model_type;

        match model_type {
            model_type if model_type == ModelType::Assignment as i32 => {
                // self.process_assignment_event(event).await
                Ok(())
            }
            model_type if model_type == ModelType::Deployment as i32 => {
                self.process_deployment_event(event)
            }
            model_type if model_type == ModelType::Host as i32 => {
                // self.process_host_event(event).await
                Ok(())
            }
            model_type if model_type == ModelType::Target as i32 => {
                // self.process_target_event(event).await
                Ok(())
            }
            model_type if model_type == ModelType::Template as i32 => {
                // self.process_template_event(event).await
                Ok(())
            }
            model_type if model_type == ModelType::Workload as i32 => {
                // self.process_workload_event(event).await
                Ok(())
            }
            model_type if model_type == ModelType::Workspace as i32 => {
                //self.process_workspace_event(event).await
                Ok(())
            }
            _ => {
                panic!("unsupported model type: {:?}", event);
            }
        }
    }

    fn compute_assignment_changes(
        deployment: &Deployment,
        current_assignments: &mut Vec<Assignment>,
        available_hosts: &mut Vec<Host>,
        desired_hosts: usize,
    ) -> anyhow::Result<(Vec<Assignment>, Vec<String>)> {
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

        Ok((assignments_to_create, assignments_to_delete))
    }

    fn process_deployment_event(&self, event: &Event) -> anyhow::Result<()> {
        // decode and load needed data
        let deployment: Deployment = DeploymentMessage::decode(&*event.serialized_model)?.into();

        let desired_hosts: usize = if event.event_type == EventType::Deleted as i32 {
            0
        } else {
            deployment.hosts as usize
        };

        let target = self.target_service.get_by_id(&deployment.target_id)?;
        let target = match target {
            Some(target) => target,
            None => {
                return Err(anyhow::anyhow!(
                    "couldn't find target with id {}",
                    deployment.target_id
                ))
            }
        };

        let mut available_hosts = self.host_service.get_matching_target(&target)?;

        let mut current_assignments = self
            .assignment_service
            .get_by_deployment_id(&deployment.id)?;

        // compute assigments to create and delete

        let (assignments_to_create, assignments_to_delete) = Self::compute_assignment_changes(
            &deployment,
            &mut current_assignments,
            &mut available_hosts,
            desired_hosts,
        )?;

        // persist changes

        for assignment in assignments_to_create {
            self.assignment_service
                .create(assignment, &event.operation_id)?;
        }

        for assignment in assignments_to_delete {
            self.assignment_service
                .delete(&assignment, &event.operation_id)?;
        }

        Ok(())
    }
}
