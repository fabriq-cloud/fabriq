use akira_core::PersistableModel;
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::{models::Assignment, persistence::AssignmentPersistence};

pub struct AssignmentMemoryPersistence {
    assignments: Arc<Mutex<HashMap<String, Assignment>>>,
}

#[async_trait]
impl AssignmentPersistence for AssignmentMemoryPersistence {
    async fn create(&self, assignment: &Assignment) -> anyhow::Result<String> {
        let mut locked_assignments = self.assignments.lock().await;

        locked_assignments.insert(assignment.get_id(), assignment.clone());

        Ok(assignment.get_id())
    }

    async fn delete(&self, assignment_id: &str) -> anyhow::Result<usize> {
        let mut locked_assignments = self.assignments.lock().await;

        locked_assignments.remove_entry(&assignment_id.to_string());

        Ok(1)
    }

    async fn get_by_deployment_id(&self, deployment_id: &str) -> anyhow::Result<Vec<Assignment>> {
        let locked_assigments = self.assignments.lock().await;

        let mut assignments_for_deployment = Vec::new();
        for assignment in (*locked_assigments).values() {
            if assignment.deployment_id == deployment_id {
                assignments_for_deployment.push(assignment.clone());
            }
        }

        Ok(assignments_for_deployment)
    }

    async fn get_by_id(&self, assignment_id: &str) -> anyhow::Result<Option<Assignment>> {
        let locked_assignments = self.assignments.lock().await;

        match locked_assignments.get(assignment_id) {
            Some(fetched_assignment) => Ok(Some(fetched_assignment.clone())),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<Assignment>> {
        let locked_assignments = self.assignments.lock().await;

        let mut assignments = Vec::new();

        for (_, assignment) in locked_assignments.iter() {
            assignments.push(assignment.clone());
        }

        Ok(assignments)
    }
}

impl Default for AssignmentMemoryPersistence {
    fn default() -> Self {
        Self {
            assignments: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        let new_assignment = Assignment {
            id: "assignment-under-test".to_owned(),
            deployment_id: "deployment-fixture".to_owned(),
            host_id: "host-fixture".to_owned(),
        };

        let assignment_persistence = AssignmentMemoryPersistence::default();

        let inserted_assignment_id = assignment_persistence
            .create(&new_assignment)
            .await
            .unwrap();
        assert_eq!(inserted_assignment_id, new_assignment.id);

        let fetched_assignment = assignment_persistence
            .get_by_id(&inserted_assignment_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_assignment.id, new_assignment.id);

        let deployment_assignments = assignment_persistence
            .get_by_deployment_id(&new_assignment.deployment_id)
            .await
            .unwrap();

        assert_eq!(deployment_assignments.len(), 1);

        let deleted_assignments = assignment_persistence
            .delete(&inserted_assignment_id)
            .await
            .unwrap();
        assert_eq!(deleted_assignments, 1);
    }
}
