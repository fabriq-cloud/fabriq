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

    pub fn compute_assignment_changes(
        deployment: &Deployment,
        existing_assignments: &[Assignment],
        target_matching_hosts: &[Host],
        desired_host_count: usize,
    ) -> anyhow::Result<(Vec<Assignment>, Vec<Assignment>)> {
        let mut assignments_to_create: Vec<Assignment> = Vec::new();
        let mut assignments_to_delete: Vec<Assignment> = Vec::new();

        let host_deleted_assignments: Vec<Assignment> = existing_assignments
            .iter()
            .filter(|assignment| {
                // if this assignment was any of the deleted, remove it.
                for host in target_matching_hosts.iter() {
                    if assignment.host_id == host.id {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        let mut assignments_after_host_check: Vec<Assignment> = existing_assignments
            .iter()
            .filter(|assignment| {
                // if this assignment was any of the deleted, remove it.
                for deleted_assignment in &host_deleted_assignments {
                    if deleted_assignment.id == assignment.id {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        let mut hosts_available: Vec<Host> = target_matching_hosts
            .iter()
            .filter(|host| {
                // if this host is already used in any of the assignments, don't reuse
                for assignment in &assignments_after_host_check {
                    if assignment.host_id == host.id {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        assignments_to_delete.extend(host_deleted_assignments);

        if assignments_after_host_check.len() > desired_host_count {
            let delete_count = assignments_after_host_check.len() - desired_host_count;

            let deleted_scale_down_assignments: Vec<Assignment> = assignments_after_host_check
                .drain(0..delete_count)
                .collect();

            assignments_to_delete.extend(deleted_scale_down_assignments);
        } else {
            let create_count = desired_host_count - assignments_after_host_check.len();

            // remove create_count hosts from available lists and use them to create assignments
            assignments_to_create = hosts_available
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

        let desired_host_count: usize = if event.event_type == EventType::Deleted as i32 {
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

        let target_matching_hosts = self.host_service.get_matching_target(&target)?;

        let existing_assignments = self
            .assignment_service
            .get_by_deployment_id(&deployment.id)?;

        // compute assigments to create and delete

        let (assignments_to_create, assignments_to_delete) = Self::compute_assignment_changes(
            &deployment,
            &existing_assignments,
            &target_matching_hosts,
            desired_host_count,
        )?;

        // persist changes

        self.assignment_service
            .create_many(&assignments_to_create, &event.operation_id)?;

        self.assignment_service
            .delete_many(&assignments_to_delete, &event.operation_id)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_deployment() {
        let deployment = Deployment {
            id: "created-deployment".to_string(),
            target_id: "target-id".to_string(),
            hosts: 1,
            workload_id: "workload-id".to_string(),
        };

        let existing_assignments: Vec<Assignment> = Vec::new();
        let target_matching_hosts = vec![
            Host {
                id: "host1-id".to_string(),
                labels: vec!["region:eastus2".to_string()],
            },
            Host {
                id: "host2-id".to_string(),
                labels: vec!["region:eastus2".to_string()],
            },
            Host {
                id: "host3-id".to_string(),
                labels: vec!["region:eastus2".to_string()],
            },
        ];

        let desired_host_count = 1;

        let (assignments_to_create, assignments_to_delete) =
            Reconciler::compute_assignment_changes(
                &deployment,
                &existing_assignments,
                &target_matching_hosts,
                desired_host_count,
            )
            .unwrap();

        assert_eq!(assignments_to_create.len(), 1);
        assert_eq!(assignments_to_delete.len(), 0);

        let assignment = assignments_to_create.first().unwrap();
        assert_eq!(assignment.deployment_id, deployment.id);
        assert_eq!(assignment.host_id, "host1-id");
    }

    #[test]
    fn test_scale_up_deployment() {
        let deployment = Deployment {
            id: "created-deployment".to_string(),
            target_id: "target-id".to_string(),
            hosts: 2,
            workload_id: "workload-id".to_string(),
        };

        let existing_assignments: Vec<Assignment> = vec![Assignment {
            id: "assignment1-id".to_string(),
            deployment_id: "deployment-id".to_string(),
            host_id: "host1-id".to_string(),
        }];

        let target_matching_hosts = vec![
            Host {
                id: "host1-id".to_string(),
                labels: vec!["region:eastus2".to_string()],
            },
            Host {
                id: "host2-id".to_string(),
                labels: vec!["region:eastus2".to_string()],
            },
        ];

        let desired_host_count = 2;

        let (assignments_to_create, assignments_to_delete) =
            Reconciler::compute_assignment_changes(
                &deployment,
                &existing_assignments,
                &target_matching_hosts,
                desired_host_count,
            )
            .unwrap();

        assert_eq!(assignments_to_create.len(), 1);
        assert_eq!(assignments_to_delete.len(), 0);

        let assignment = assignments_to_create.first().unwrap();
        assert_eq!(assignment.deployment_id, deployment.id);
        assert_eq!(assignment.host_id, "host2-id");
    }

    #[test]
    fn test_scale_down_deployment() {
        let deployment = Deployment {
            id: "deployment-id".to_string(),
            target_id: "target-id".to_string(),
            hosts: 2,
            workload_id: "workload-id".to_string(),
        };

        let existing_assignments: Vec<Assignment> = vec![
            Assignment {
                id: "assignment1-id".to_string(),
                deployment_id: "deployment-id".to_string(),
                host_id: "host1-id".to_string(),
            },
            Assignment {
                id: "assignment2-id".to_string(),
                deployment_id: "deployment-id".to_string(),
                host_id: "host2-id".to_string(),
            },
        ];

        let target_matching_hosts = vec![
            Host {
                id: "host1-id".to_string(),
                labels: vec!["region:eastus2".to_string()],
            },
            Host {
                id: "host2-id".to_string(),
                labels: vec!["region:eastus2".to_string()],
            },
        ];

        let desired_host_count = 1;

        let (assignments_to_create, assignments_to_delete) =
            Reconciler::compute_assignment_changes(
                &deployment,
                &existing_assignments,
                &target_matching_hosts,
                desired_host_count,
            )
            .unwrap();

        assert_eq!(assignments_to_create.len(), 0);
        assert_eq!(assignments_to_delete.len(), 1);

        let assignment = assignments_to_delete.first().unwrap();
        assert_eq!(assignment.deployment_id, deployment.id);
        assert_eq!(assignment.host_id, "host1-id");
    }

    #[test]
    fn test_host_deleted_deployment() {
        let deployment = Deployment {
            id: "deployment-id".to_string(),
            target_id: "target-id".to_string(),
            hosts: 2,
            workload_id: "workload-id".to_string(),
        };

        let existing_assignments: Vec<Assignment> = vec![
            Assignment {
                id: "assignment1-id".to_string(),
                deployment_id: "deployment-id".to_string(),
                host_id: "host1-id".to_string(),
            },
            Assignment {
                id: "assignment2-id".to_string(),
                deployment_id: "deployment-id".to_string(),
                host_id: "host2-id".to_string(),
            },
        ];

        let target_matching_hosts = vec![Host {
            id: "host1-id".to_string(),
            labels: vec!["region:eastus2".to_string()],
        }];

        let desired_host_count = 0;

        let (assignments_to_create, assignments_to_delete) =
            Reconciler::compute_assignment_changes(
                &deployment,
                &existing_assignments,
                &target_matching_hosts,
                desired_host_count,
            )
            .unwrap();

        assert_eq!(assignments_to_create.len(), 0);
        assert_eq!(assignments_to_delete.len(), 2);

        let delete_assignment = &assignments_to_delete[0];
        assert_eq!(delete_assignment.deployment_id, deployment.id);
        assert_eq!(delete_assignment.host_id, "host2-id");

        let delete_assignment = &assignments_to_delete[1];
        assert_eq!(delete_assignment.deployment_id, deployment.id);
        assert_eq!(delete_assignment.host_id, "host1-id");
    }
}
