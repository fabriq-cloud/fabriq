// CoreProcessor reacts to events on models and creates / updates / or deletes other models in reaction.
// For example, if a deployment is created, it fetches its target, the hosts in the system, and creates assignments.
// It only handles work that is common across processors to keep the object model coherent.
// It does not handle the work that is specific to a specific workflow, for example truing up a GitOps repo.
// These processor specific workflows are implemented using external processors.

use prost::Message;
use std::{cmp, collections::HashMap, sync::Arc};

use crate::{
    models::{Assignment, Deployment, Host, Target, Template, Workload},
    services::{
        AssignmentService, DeploymentService, HostService, TargetService, TemplateService,
        WorkloadService,
    },
};
use fabriq_core::{
    AssignmentMessage, DeploymentMessage, Event, EventType, HostMessage, ModelType, OperationId,
    TargetMessage, TemplateMessage, WorkloadMessage,
};

#[derive(Debug)]
pub struct Reconciler {
    pub assignment_service: Arc<AssignmentService>,
    pub deployment_service: Arc<DeploymentService>,
    pub host_service: Arc<HostService>,
    pub target_service: Arc<TargetService>,
    pub template_service: Arc<TemplateService>,
    pub workload_service: Arc<WorkloadService>,
}

impl Reconciler {
    #[tracing::instrument]
    pub async fn process(&self, event: &Event) -> anyhow::Result<()> {
        let model_type = event.model_type;

        match model_type {
            model_type if model_type == ModelType::Assignment as i32 => {
                self.process_assignment_event(event).await
            }
            model_type if model_type == ModelType::Config as i32 => {
                self.process_config_event(event).await
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
            _ => {
                let msg = format!("unhandled model type: {}", model_type);
                tracing::error!(msg);

                Err(anyhow::anyhow!(msg))
            }
        }
    }

    #[tracing::instrument]
    async fn process_assignment_event(&self, event: &Event) -> anyhow::Result<()> {
        Ok(())
    }

    #[tracing::instrument]
    async fn process_config_event(&self, event: &Event) -> anyhow::Result<()> {
        Ok(())
    }

    #[tracing::instrument]
    async fn process_workload_event(&self, event: &Event) -> anyhow::Result<()> {
        let workload = Self::get_current_or_previous_model::<WorkloadMessage, Workload>(event)?;
        let deployments = self
            .deployment_service
            .get_by_workload_id(&workload.id)
            .await?;
        for deployment in deployments {
            self.process_deployment_event_impl(
                &deployment,
                deployment.host_count as usize,
                &event.operation_id,
            )
            .await?;
        }

        Ok(())
    }

    #[tracing::instrument]
    async fn process_deployment_event(&self, event: &Event) -> anyhow::Result<()> {
        let deployment =
            Self::get_current_or_previous_model::<DeploymentMessage, Deployment>(event)?;

        let desired_host_count: usize = if event.event_type == EventType::Deleted as i32 {
            0
        } else {
            deployment.host_count as usize
        };

        self.process_deployment_event_impl(&deployment, desired_host_count, &event.operation_id)
            .await
    }

    pub async fn process_deployment_event_impl(
        &self,
        deployment: &Deployment,
        desired_host_count: usize,
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<()> {
        let target = self.target_service.get_by_id(&deployment.target_id).await?;
        let target = match target {
            Some(target) => target,
            None => {
                return Err(anyhow::anyhow!(
                    "couldn't find deployment target with id {}",
                    deployment.target_id
                ))
            }
        };

        let target_matching_hosts = self.host_service.get_matching_target(&target).await?;

        let existing_assignments = self
            .assignment_service
            .get_by_deployment_id(&deployment.id)
            .await?;

        // compute assigments to create and delete
        let (assignments_to_create, assignments_to_delete) = Self::compute_assignment_changes(
            deployment,
            &existing_assignments,
            &target_matching_hosts,
            desired_host_count,
        )?;

        // persist changes
        self.assignment_service
            .upsert_many(&assignments_to_create, operation_id)
            .await?;

        self.assignment_service
            .delete_many(&assignments_to_delete, operation_id)
            .await?;

        Ok(())
    }

    #[tracing::instrument]
    async fn process_host_event(&self, event: &Event) -> anyhow::Result<()> {
        let mut spanning_target_set: HashMap<String, Target> = HashMap::new();

        if let Some(serialized_previous_model) = event.serialized_previous_model.clone() {
            let previous_host = HostMessage::decode(&*serialized_previous_model)?.into();
            let previous_targets = self
                .target_service
                .get_matching_host(&previous_host)
                .await?;

            for target in previous_targets {
                spanning_target_set.insert(target.id.clone(), target);
            }
        }

        if let Some(serialized_current_model) = event.serialized_current_model.clone() {
            let current_host = HostMessage::decode(&*serialized_current_model)?.into();
            let current_targets = self.target_service.get_matching_host(&current_host).await?;

            for target in current_targets {
                spanning_target_set.insert(target.id.clone(), target);
            }
        }

        let spanning_targets = spanning_target_set.values().cloned().collect::<Vec<_>>();

        self.update_deployments_for_targets(&spanning_targets, &event.operation_id)
            .await?;

        Ok(())
    }

    async fn update_deployments_for_targets(
        &self,
        targets: &[Target],
        operation_id: &Option<OperationId>,
    ) -> anyhow::Result<()> {
        for target in targets {
            let deployments = self.deployment_service.get_by_target_id(&target.id).await?;

            for deployment in deployments {
                self.process_deployment_event_impl(
                    &deployment,
                    deployment.host_count as usize,
                    operation_id,
                )
                .await?;
            }
        }

        Ok(())
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

    #[tracing::instrument]
    async fn process_template_event(&self, event: &Event) -> anyhow::Result<()> {
        let template = Self::get_current_or_previous_model::<TemplateMessage, Template>(event)?;
        let mut spanning_deployments_set: HashMap<String, Deployment> = HashMap::new();

        let workloads = self
            .workload_service
            .get_by_template_id(&template.id)
            .await?;
        for workload in workloads {
            let deployments = self
                .deployment_service
                .get_by_workload_id(&workload.id)
                .await?;
            for deployment in deployments {
                // we will pull in deployments with this template_id as an override below
                if deployment.template_id.is_none() {
                    spanning_deployments_set.insert(deployment.id.clone(), deployment);
                }
            }
        }

        let deployments = self
            .deployment_service
            .get_by_template_id(&template.id)
            .await?;

        for deployment in deployments {
            spanning_deployments_set.insert(deployment.id.clone(), deployment);
        }

        let spanning_deployments = spanning_deployments_set
            .values()
            .cloned()
            .collect::<Vec<_>>();

        for deployment in spanning_deployments {
            self.process_deployment_event_impl(
                &deployment,
                deployment.host_count as usize,
                &event.operation_id,
            )
            .await?;
        }

        Ok(())

        // self.update_deployments_for_targets(&spanning_targets, &event.operation_id)
    }

    #[tracing::instrument]
    async fn process_target_event(&self, event: &Event) -> anyhow::Result<()> {
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
            .await
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
            let create_count = cmp::min(
                hosts_available.len(),
                desired_host_count - assignments_after_host_check.len(),
            );

            // remove create_count hosts from available lists and use them to create assignments
            assignments_to_create = hosts_available
                .drain(0..create_count)
                .map(|host| Assignment {
                    id: AssignmentMessage::make_id(&deployment.id, &host.id),
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
    use crate::{
        models::Template,
        persistence::memory::{
            AssignmentMemoryPersistence, ConfigMemoryPersistence, DeploymentMemoryPersistence,
            HostMemoryPersistence, MemoryPersistence, WorkloadMemoryPersistence,
        },
        services::ConfigService,
    };
    use fabriq_core::{
        create_event,
        test::{
            get_deployment_fixture, get_host_fixture, get_target_fixture, get_template_fixture,
            get_workload_fixture,
        },
        EventStream,
    };
    use fabriq_memory_stream::MemoryEventStream;

    use super::*;

    async fn create_reconciler_fixture() -> anyhow::Result<Reconciler> {
        let event_stream: Arc<dyn EventStream> = Arc::new(MemoryEventStream::new()?);

        let assignment_persistence = Box::<AssignmentMemoryPersistence>::default();
        let assignment_service = Arc::new(AssignmentService {
            persistence: assignment_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let target_persistence = Box::<MemoryPersistence<Target>>::default();
        let target_service = Arc::new(TargetService {
            persistence: target_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let target: Target = get_target_fixture(Some("target-fixture")).into();
        target_service.upsert(&target, &None).await.unwrap();

        let config_persistence = ConfigMemoryPersistence::default();
        let config_service = Arc::new(ConfigService {
            persistence: Box::new(config_persistence),
            event_stream: Arc::clone(&event_stream),
        });

        let deployment_persistence = Box::<DeploymentMemoryPersistence>::default();
        let deployment_service = Arc::new(DeploymentService {
            persistence: deployment_persistence,
            event_stream: Arc::clone(&event_stream),

            assignment_service: Arc::clone(&assignment_service),
            config_service: Arc::clone(&config_service),
            target_service: Arc::clone(&target_service),
        });

        let host_persistence = Box::<HostMemoryPersistence>::default();
        let host_service = Arc::new(HostService {
            persistence: host_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let template_persistence = Box::<MemoryPersistence<Template>>::default();
        let template_service = Arc::new(TemplateService {
            persistence: template_persistence,
            event_stream: Arc::clone(&event_stream),
        });

        let workload_persistence = Box::<WorkloadMemoryPersistence>::default();
        let workload_service = Arc::new(WorkloadService {
            persistence: workload_persistence,
            event_stream: Arc::clone(&event_stream),

            template_service: Arc::clone(&template_service),
        });

        let reconciler = Reconciler {
            assignment_service,
            deployment_service,
            host_service,
            target_service,
            template_service,
            workload_service,
        };

        let host1 = get_host_fixture(Some("host1-id")).into();
        reconciler.host_service.upsert(&host1, &None).await.unwrap();

        let host2 = Host {
            id: "host2-id".to_owned(),
            labels: vec!["region:westus2".to_owned(), "cloud:azure".to_owned()],
        };
        reconciler.host_service.upsert(&host2, &None).await.unwrap();

        let host3 = get_host_fixture(Some("host3-id")).into();
        reconciler.host_service.upsert(&host3, &None).await.unwrap();

        let deployment = get_deployment_fixture(None).into();
        reconciler
            .deployment_service
            .upsert(&deployment, &None)
            .await
            .unwrap();

        let target = get_target_fixture(None).into();
        reconciler
            .target_service
            .upsert(&target, &None)
            .await
            .unwrap();

        let template = get_template_fixture(None).into();
        reconciler
            .template_service
            .upsert(&template, None)
            .await
            .unwrap();

        Ok(reconciler)
    }

    #[tokio::test]
    async fn test_process_deployment_event() {
        let reconciler = create_reconciler_fixture().await.unwrap();

        let operation_id = OperationId::create();

        let deployment = get_deployment_fixture(None);

        let event = create_event::<DeploymentMessage>(
            &None,
            &Some(deployment),
            EventType::Created,
            ModelType::Deployment,
            &operation_id,
        );

        reconciler.process(&event).await.unwrap();

        let assignments = reconciler.assignment_service.list().await.unwrap();

        assert_eq!(assignments.len(), 2);
    }

    #[tokio::test]
    async fn test_process_host_event() {
        let reconciler = create_reconciler_fixture().await.unwrap();

        let host4 = Host {
            id: "host4-id".to_owned(),
            labels: vec!["region:westus2".to_owned(), "cloud:azure".to_owned()],
        };

        let operation_id = OperationId::create();

        let event = create_event::<HostMessage>(
            &None,
            &Some(host4.into()),
            EventType::Created,
            ModelType::Host,
            &operation_id,
        );

        reconciler.process(&event).await.unwrap();

        let assignments = reconciler.assignment_service.list().await.unwrap();

        assert_eq!(assignments.len(), 0);

        let host3 = Host {
            id: "host3-id".to_owned(),
            labels: vec!["region:eastus2".to_owned(), "cloud:azure".to_owned()],
        };

        let event = create_event::<HostMessage>(
            &None,
            &Some(host3.into()),
            EventType::Created,
            ModelType::Host,
            &operation_id,
        );

        reconciler.process(&event).await.unwrap();

        let assignments = reconciler.assignment_service.list().await.unwrap();

        assert_eq!(assignments.len(), 2);

        for assignment in assignments {
            assert!(assignment.host_id == "host1-id" || assignment.host_id == "host3-id");
        }
    }

    #[tokio::test]
    async fn test_process_target_event() {
        let reconciler = create_reconciler_fixture().await.unwrap();

        let target_message = get_target_fixture(None);

        let operation_id = OperationId::create();

        let event = create_event::<TargetMessage>(
            &None,
            &Some(target_message),
            EventType::Created,
            ModelType::Target,
            &operation_id,
        );

        reconciler.process(&event).await.unwrap();

        let assignments = reconciler.assignment_service.list().await.unwrap();

        assert_eq!(assignments.len(), 2);
    }

    #[tokio::test]
    async fn test_process_workload_event() {
        let reconciler = create_reconciler_fixture().await.unwrap();

        let workload = get_workload_fixture(None).into();
        reconciler
            .workload_service
            .upsert(&workload, None)
            .await
            .unwrap();

        let operation_id = OperationId::create();

        let event = create_event::<WorkloadMessage>(
            &None,
            &Some(workload.into()),
            EventType::Created,
            ModelType::Workload,
            &operation_id,
        );

        reconciler.process(&event).await.unwrap();

        let assignments = reconciler.assignment_service.list().await.unwrap();

        assert_eq!(assignments.len(), 2);
    }

    #[tokio::test]
    async fn test_process_template_event() {
        let reconciler = create_reconciler_fixture().await.unwrap();

        let operation_id = OperationId::create();

        let template = get_template_fixture(None);

        let event = create_event::<TemplateMessage>(
            &None,
            &Some(template),
            EventType::Created,
            ModelType::Template,
            &operation_id,
        );

        reconciler.process(&event).await.unwrap();

        let assignments = reconciler.assignment_service.list().await.unwrap();

        assert_eq!(assignments.len(), 2);
    }

    #[test]
    fn test_new_deployment() {
        let deployment = get_deployment_fixture(None).into();

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
        let deployment: Deployment = get_deployment_fixture(None).into();

        let existing_assignments: Vec<Assignment> = vec![Assignment {
            id: AssignmentMessage::make_id(&deployment.id, "host1-id"),
            deployment_id: deployment.id.to_string(),
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

        // test that assignment processing handles case where more hosts are desired than are available
        let desired_host_count = usize::MAX;

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
        let deployment: Deployment = get_deployment_fixture(None).into();

        let existing_assignments: Vec<Assignment> = vec![
            Assignment {
                id: AssignmentMessage::make_id(&deployment.id, "host1-id"),
                deployment_id: deployment.id.to_string(),
                host_id: "host1-id".to_string(),
            },
            Assignment {
                id: AssignmentMessage::make_id(&deployment.id, "host2-id"),
                deployment_id: deployment.id.to_string(),
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
        let deployment: Deployment = get_deployment_fixture(None).into();

        let existing_assignments: Vec<Assignment> = vec![
            Assignment {
                id: AssignmentMessage::make_id(&deployment.id, "host1-id"),
                deployment_id: deployment.id.to_string(),
                host_id: "host1-id".to_string(),
            },
            Assignment {
                id: AssignmentMessage::make_id(&deployment.id, "host2-id"),
                deployment_id: deployment.id.to_string(),
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
        let deployment: Deployment = get_deployment_fixture(None).into();

        let existing_assignments: Vec<Assignment> = vec![Assignment {
            id: AssignmentMessage::make_id(&deployment.id, "host1-id"),
            deployment_id: deployment.id.to_string(),
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
