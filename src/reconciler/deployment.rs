use super::Reconciler;

impl Reconciler {
    async fn process_deployment_event(&self, event: &Event) -> anyhow::Result<()> {
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
                .create(assignment, &event.operation_id)?;
        }

        for assignment in assignments_to_delete {
            self.assignment_service
                .delete(&assignment, &event.operation_id)?;
        }

        Ok(())
    }
}
