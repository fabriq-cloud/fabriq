// CoreProcessor reacts to events on models and creates / updates / or deletes other models in reaction.
// For example, if a deployment is created, it fetches its target, the hosts in the system, and creates assignments.
// It only handles work that is common across processors to keep the object model coherent.
// It does not handle the work that is specific to a specific workflow, for example truing up a GitOps repo.
// These processor specific workflows are implemented using external processors.

use std::{collections::HashMap, sync::Arc};

use akira_core::{
    DeploymentMessage, Event, EventType, HostMessage, ModelType, OperationId, TargetMessage,
};
use prost::Message;

use akira::{
    models::{Assignment, Deployment, Host, Target},
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
            model_type if model_type == ModelType::Assignment as i32 => Ok(()),
            model_type if model_type == ModelType::Deployment as i32 => {
                self.process_deployment_event(event)
            }
            model_type if model_type == ModelType::Host as i32 => self.process_host_event(event),
            model_type if model_type == ModelType::Target as i32 => {
                self.process_target_event(event)
            }
            model_type if model_type == ModelType::Template as i32 => {
                // self.process_template_event(event)
                Ok(())
            }
            model_type if model_type == ModelType::Workload as i32 => {
                // self.process_workload_event(event)
                Ok(())
            }
            model_type if model_type == ModelType::Workspace as i32 => {
                //self.process_workspace_event(event)
                Ok(())
            }
            _ => {
                panic!("unsupported model type: {:?}", event);
            }
        }
    }

    fn process_deployment_event(&self, event: &Event) -> anyhow::Result<()> {
        let deployment_option = if let Some(serialized_previous_model) =
            &event.serialized_current_model
        {
            let deployment_message =
                DeploymentMessage::decode(&*serialized_previous_model.clone())?;
            Some(deployment_message.into())
        } else if let Some(serialized_current_model) = &event.serialized_current_model {
            let deployment_message = DeploymentMessage::decode(&*serialized_current_model.clone())?;
            Some(deployment_message.into())
        } else {
            None
        };

        let deployment: Deployment = match deployment_option {
            Some(deployment) => deployment,
            None => {
                return Err(anyhow::anyhow!(
                    "Event received without previous or current deployment {:?}",
                    event
                ))
            }
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

        let desired_host_count: usize = if event.event_type == EventType::Deleted as i32 {
            0
        } else {
            deployment.hosts as usize
        };

        self.process_deployment_event_impl(
            &deployment,
            &target,
            desired_host_count,
            &event.operation_id,
        )
    }

    pub fn process_deployment_event_impl(
        &self,
        deployment: &Deployment,
        target: &Target,
        desired_host_count: usize,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<()> {
        let target_matching_hosts = self.host_service.get_matching_target(target)?;

        let existing_assignments = self
            .assignment_service
            .get_by_deployment_id(&deployment.id)?;

        // compute assigments to create and delete

        let (assignments_to_create, assignments_to_delete) = Self::compute_assignment_changes(
            deployment,
            &existing_assignments,
            &target_matching_hosts,
            desired_host_count,
        )?;

        // persist changes

        self.assignment_service
            .create_many(&assignments_to_create, operation_id)?;

        self.assignment_service
            .delete_many(&assignments_to_delete, operation_id)?;

        Ok(())
    }

    fn process_host_event(&self, event: &Event) -> anyhow::Result<()> {
        let mut spanning_target_set: HashMap<String, Target> = HashMap::new();

        if let Some(serialized_previous_model) = event.serialized_previous_model.clone() {
            let previous_host = HostMessage::decode(&*serialized_previous_model)?.into();
            let previous_targets = self.target_service.get_matching_host(&previous_host)?;

            for target in previous_targets {
                spanning_target_set.insert(target.id.clone(), target);
            }
        }

        if let Some(serialized_current_model) = event.serialized_current_model.clone() {
            let current_host = HostMessage::decode(&*serialized_current_model)?.into();
            let current_targets = self.target_service.get_matching_host(&current_host)?;

            for target in current_targets {
                spanning_target_set.insert(target.id.clone(), target);
            }
        }

        let spanning_targets = spanning_target_set.values().cloned().collect::<Vec<_>>();

        self.update_deployments_for_targets(&spanning_targets, &event.operation_id)?;

        Ok(())
    }

    fn update_deployments_for_targets(
        &self,
        targets: &[Target],
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<()> {
        for target in targets {
            let deployments = self.deployment_service.get_by_target_id(&target.id)?;

            for deployment in deployments {
                self.process_deployment_event_impl(
                    &deployment,
                    &target,
                    deployment.hosts as usize,
                    operation_id,
                )?;
            }
        }

        Ok(())
    }

    fn process_target_event(&self, event: &Event) -> anyhow::Result<()> {
        let mut spanning_targets: Vec<Target> = Vec::new();

        if let Some(serialized_previous_model) = event.serialized_previous_model.clone() {
            let previous_target = TargetMessage::decode(&*serialized_previous_model)?.into();
            spanning_targets.push(previous_target);
        }

        if let Some(serialized_current_model) = event.serialized_current_model.clone() {
            let current_target = TargetMessage::decode(&*serialized_current_model)?.into();
            spanning_targets.push(current_target);
        }

        self.update_deployments_for_targets(&spanning_targets, &event.operation_id)
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
}

#[cfg(test)]
mod tests {
    use akira::models::{Template, Workload, Workspace};
    use akira::persistence::memory::{
        AssignmentMemoryPersistence, DeploymentMemoryPersistence, HostMemoryPersistence,
        MemoryPersistence,
    };
    use akira_core::EventStream;
    use akira_memory_stream::MemoryEventStream;

    use super::*;

    fn create_reconciler_fixture() -> anyhow::Result<Reconciler> {
        let event_stream: Arc<Box<dyn EventStream>> = Arc::new(Box::new(MemoryEventStream::new()?));

        let assignment_persistence = Box::new(AssignmentMemoryPersistence::default());
        let assignment_service = Arc::new(AssignmentService {
            persistence: assignment_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let deployment_persistence = Box::new(DeploymentMemoryPersistence::default());
        let deployment_service = Arc::new(DeploymentService {
            persistence: deployment_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let host_persistence = Box::new(HostMemoryPersistence::default());
        let host_service = Arc::new(HostService {
            persistence: host_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let target_persistence = Box::new(MemoryPersistence::<Target>::default());
        let target_service = Arc::new(TargetService {
            persistence: target_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let template_persistence = Box::new(MemoryPersistence::<Template>::default());
        let template_service = Arc::new(TemplateService {
            persistence: template_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let workload_persistence = Box::new(MemoryPersistence::<Workload>::default());
        let workload_service = Arc::new(WorkloadService {
            persistence: workload_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let workspace_persistence = Box::new(MemoryPersistence::<Workspace>::default());
        let workspace_service = Arc::new(WorkspaceService {
            persistence: workspace_persistence,
            event_stream: Arc::clone(&event_stream),

            workload_service: Arc::clone(&workload_service),
        });

        Ok(Reconciler {
            assignment_service,
            deployment_service,
            host_service,
            target_service,
            template_service,
            workload_service,
            workspace_service,
        })
    }

    #[test]
    fn test_process_target_event() {
        let reconciler = create_reconciler_fixture().unwrap();

        let host1 = Host {
            id: "host1-id".to_owned(),
            labels: vec!["region:eastus2".to_owned(), "cloud:azure".to_owned()],
        };

        reconciler.host_service.create(&host1, &None).unwrap();

        let host2 = Host {
            id: "host3-id".to_owned(),
            labels: vec!["region:westus2".to_owned(), "cloud:azure".to_owned()],
        };

        reconciler.host_service.create(&host2, &None).unwrap();

        let host3 = Host {
            id: "host3-id".to_owned(),
            labels: vec!["region:eastus2".to_owned(), "cloud:azure".to_owned()],
        };

        reconciler.host_service.create(&host3, &None).unwrap();

        let deployment = Deployment {
            id: "deployment-fixture".to_owned(),
            target_id: "eastus2".to_owned(),
            hosts: 2,
            workload_id: "workload-fixture".to_owned(),
        };

        let operation_id = OperationId::create();

        reconciler
            .deployment_service
            .create(&deployment, &Some(operation_id.clone()))
            .unwrap();

        let target = Target {
            id: "eastus2".to_owned(),
            labels: vec!["region:eastus2".to_owned()],
        };

        reconciler.target_service.create(&target, &None).unwrap();

        let event = akira::services::TargetService::create_event(
            &None,
            &Some(target),
            EventType::Created,
            &operation_id,
        );

        reconciler.process(&event).unwrap();

        let assignments = reconciler.assignment_service.list().unwrap();

        assert_eq!(assignments.len(), 2);
    }

    #[test]
    fn test_process_deployment_event() {
        let reconciler = create_reconciler_fixture().unwrap();

        let host1 = Host {
            id: "host1-id".to_owned(),
            labels: vec!["region:eastus2".to_owned(), "cloud:azure".to_owned()],
        };

        reconciler.host_service.create(&host1, &None).unwrap();

        let host2 = Host {
            id: "host3-id".to_owned(),
            labels: vec!["region:westus2".to_owned(), "cloud:azure".to_owned()],
        };

        reconciler.host_service.create(&host2, &None).unwrap();

        let host3 = Host {
            id: "host3-id".to_owned(),
            labels: vec!["region:eastus2".to_owned(), "cloud:azure".to_owned()],
        };

        reconciler.host_service.create(&host3, &None).unwrap();

        let target = Target {
            id: "eastus2".to_owned(),
            labels: vec!["region:eastus2".to_owned()],
        };

        reconciler.target_service.create(&target, &None).unwrap();

        let deployment = Deployment {
            id: "deployment-fixture".to_owned(),
            target_id: "eastus2".to_owned(),
            hosts: 2,
            workload_id: "workload-fixture".to_owned(),
        };

        let operation_id = OperationId::create();

        reconciler
            .deployment_service
            .create(&deployment, &Some(operation_id.clone()))
            .unwrap();

        let event = akira::services::DeploymentService::create_event(
            &None,
            &Some(deployment),
            EventType::Created,
            &operation_id,
        );

        reconciler.process(&event).unwrap();

        let assignments = reconciler.assignment_service.list().unwrap();

        assert_eq!(assignments.len(), 2);
    }

    #[test]
    fn test_process_host_event() {
        let reconciler = create_reconciler_fixture().unwrap();

        let host1 = Host {
            id: "host1-id".to_owned(),
            labels: vec!["region:eastus2".to_owned(), "cloud:azure".to_owned()],
        };

        reconciler.host_service.create(&host1, &None).unwrap();

        let host2 = Host {
            id: "host3-id".to_owned(),
            labels: vec!["region:westus2".to_owned(), "cloud:azure".to_owned()],
        };

        reconciler.host_service.create(&host2, &None).unwrap();

        let host3 = Host {
            id: "host3-id".to_owned(),
            labels: vec!["region:eastus2".to_owned(), "cloud:azure".to_owned()],
        };

        reconciler.host_service.create(&host3, &None).unwrap();

        let host4 = Host {
            id: "host4-id".to_owned(),
            labels: vec!["region:westus2".to_owned(), "cloud:azure".to_owned()],
        };

        reconciler.host_service.create(&host4, &None).unwrap();

        let target = Target {
            id: "eastus2".to_owned(),
            labels: vec!["region:eastus2".to_owned()],
        };

        reconciler.target_service.create(&target, &None).unwrap();

        let deployment = Deployment {
            id: "deployment-fixture".to_owned(),
            target_id: "eastus2".to_owned(),
            hosts: 2,
            workload_id: "workload-fixture".to_owned(),
        };

        let operation_id = OperationId::create();

        reconciler
            .deployment_service
            .create(&deployment, &Some(operation_id.clone()))
            .unwrap();

        let event = akira::services::HostService::create_event(
            &None,
            &Some(host4),
            EventType::Created,
            &operation_id,
        );

        reconciler.process(&event).unwrap();

        let assignments = reconciler.assignment_service.list().unwrap();

        assert_eq!(assignments.len(), 0);

        let event = akira::services::HostService::create_event(
            &None,
            &Some(host3),
            EventType::Created,
            &operation_id,
        );

        reconciler.process(&event).unwrap();

        let assignments = reconciler.assignment_service.list().unwrap();

        assert_eq!(assignments.len(), 2);

        for assignment in assignments {
            assert!(assignment.host_id == "host1-id" || assignment.host_id == "host3-id");
        }
    }

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

    #[test]
    fn test_do_nothing_deployment() {
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
        assert_eq!(assignments_to_delete.len(), 0);
    }
}
