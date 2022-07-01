use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    models::Assignment,
    persistence::{AssignmentPersistence, PersistableModel, Persistence},
};

pub struct AssignmentMemoryPersistence {
    models: Arc<Mutex<HashMap<String, Assignment>>>,
}

impl Persistence<Assignment> for AssignmentMemoryPersistence {
    fn create(&self, assignment: &Assignment) -> anyhow::Result<String> {
        let mut locked_assignments = self.get_models_locked()?;

        locked_assignments.insert(assignment.get_id(), assignment.clone());

        Ok(assignment.get_id())
    }

    fn create_many(&self, assignments: &[Assignment]) -> anyhow::Result<Vec<String>> {
        let mut assignment_ids = Vec::new();
        for (_, assignment) in assignments.iter().enumerate() {
            let assignment_id = self.create(assignment)?;
            assignment_ids.push(assignment_id);
        }

        Ok(assignment_ids)
    }

    fn delete(&self, assignment_id: &str) -> anyhow::Result<usize> {
        let mut locked_assignments = self.get_models_locked()?;

        locked_assignments.remove_entry(&assignment_id.to_string());

        Ok(1)
    }

    fn delete_many(&self, assignment_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, assignment_id) in assignment_ids.iter().enumerate() {
            self.delete(assignment_id)?;
        }

        Ok(assignment_ids.len())
    }

    fn get_by_id(&self, assignment_id: &str) -> anyhow::Result<Option<Assignment>> {
        let locked_assignments = self.get_models_locked()?;

        match locked_assignments.get(assignment_id) {
            Some(fetched_assignment) => Ok(Some(fetched_assignment.clone())),
            None => Ok(None),
        }
    }

    fn list(&self) -> anyhow::Result<Vec<Assignment>> {
        let locked_assignments = self.get_models_locked()?;

        let assignments = locked_assignments.values().cloned().collect();

        Ok(assignments)
    }
}

impl AssignmentPersistence for AssignmentMemoryPersistence {
    fn get_by_deployment_id(&self, deployment_id: &str) -> anyhow::Result<Vec<Assignment>> {
        let locked_assignments = self.get_models_locked()?;

        let mut assignments_for_deployment = Vec::new();
        for assignment in (*locked_assignments).values() {
            if assignment.deployment_id == deployment_id {
                assignments_for_deployment.push(assignment.clone());
            }
        }

        Ok(assignments_for_deployment)
    }
}

impl AssignmentMemoryPersistence {
    fn get_models_locked(&self) -> anyhow::Result<MutexGuard<HashMap<String, Assignment>>> {
        match self.models.lock() {
            Ok(locked_assignments) => Ok(locked_assignments),
            Err(_) => Err(anyhow::anyhow!("failed to acquire lock")),
        }
    }
}

impl Default for AssignmentMemoryPersistence {
    fn default() -> Self {
        Self {
            models: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();

        let new_assignment = Assignment {
            id: "assignment-under-test".to_owned(),
            deployment_id: "deployment-fixture".to_owned(),
            host_id: "host-fixture".to_owned(),
        };

        let assignment_persistence = AssignmentMemoryPersistence::default();

        let inserted_assignment_id = assignment_persistence.create(&new_assignment).unwrap();
        assert_eq!(inserted_assignment_id, new_assignment.id);

        let fetched_assignment = assignment_persistence
            .get_by_id(&inserted_assignment_id)
            .unwrap()
            .unwrap();

        assert_eq!(fetched_assignment.id, new_assignment.id);

        let deployment_assignments = assignment_persistence
            .get_by_deployment_id(&new_assignment.deployment_id)
            .unwrap();

        assert_eq!(deployment_assignments.len(), 1);

        let deleted_assignments = assignment_persistence
            .delete(&inserted_assignment_id)
            .unwrap();
        assert_eq!(deleted_assignments, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();

        let new_assignment = Assignment {
            id: "assignment-under-test".to_owned(),
            deployment_id: "deployment-fixture".to_owned(),
            host_id: "host-fixture".to_owned(),
        };

        let assignment_persistence = AssignmentMemoryPersistence::default();

        let inserted_host_ids = assignment_persistence
            .create_many(&[new_assignment.clone()])
            .unwrap();
        assert_eq!(inserted_host_ids.len(), 1);
        assert_eq!(inserted_host_ids[0], new_assignment.id);

        let deleted_hosts = assignment_persistence
            .delete_many(&[&new_assignment.id])
            .unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
