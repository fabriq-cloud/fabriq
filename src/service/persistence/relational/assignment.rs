use diesel::prelude::*;

use crate::persistence::{AssignmentPersistence, Persistence};
use crate::schema::assignments::table;
use crate::{models::Assignment, schema::assignments, schema::assignments::dsl::*};

#[derive(Default)]
pub struct AssignmentRelationalPersistence {}

impl Persistence<Assignment> for AssignmentRelationalPersistence {
    fn create(&self, assignment: &Assignment) -> anyhow::Result<String> {
        let connection = crate::db::get_connection()?;

        let results: Vec<String> = diesel::insert_into(table)
            .values(assignment)
            .returning(assignments::id)
            .get_results(&connection)?;

        match results.first() {
            Some(assignment_id) => Ok(assignment_id.clone()),
            None => Err(anyhow::anyhow!("Couldn't find created host id returned")),
        }
    }

    fn create_many(&self, models: &[Assignment]) -> anyhow::Result<Vec<String>> {
        let connection = crate::db::get_connection()?;

        let results = diesel::insert_into(table)
            .values(models)
            .returning(assignments::id)
            .get_results(&connection)?;

        Ok(results)
    }

    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(assignments.filter(id.eq(model_id))).execute(&connection)?)
    }

    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, model_id) in model_ids.iter().enumerate() {
            self.delete(model_id)?;
        }

        Ok(model_ids.len())
    }

    fn get_by_id(&self, assignment_id: &str) -> anyhow::Result<Option<Assignment>> {
        let connection = crate::db::get_connection()?;

        let results = assignments
            .filter(id.eq(assignment_id))
            .load::<Assignment>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }

    fn list(&self) -> anyhow::Result<Vec<Assignment>> {
        let connection = crate::db::get_connection()?;

        let results = assignments.load::<Assignment>(&connection).unwrap();

        Ok(results)
    }
}

impl AssignmentPersistence for AssignmentRelationalPersistence {
    fn get_by_deployment_id(&self, deploy_id: &str) -> anyhow::Result<Vec<Assignment>> {
        let connection = crate::db::get_connection()?;

        let results = assignments
            .filter(deployment_id.eq(deploy_id))
            .load::<Assignment>(&connection)?;

        Ok(results)
    }
}
#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;
    use crate::models::Assignment;

    #[test]
    fn test_create_get_delete() {
        dotenv().ok();
        crate::persistence::relational::ensure_fixtures();

        let new_assignment = Assignment {
            id: "assignment-under-test".to_owned(),
            deployment_id: "deployment-fixture".to_owned(),
            host_id: "host-fixture".to_owned(),
        };

        let assignment_persistence = AssignmentRelationalPersistence::default();

        // delete assignment if it exists
        let _ = assignment_persistence.delete(&new_assignment.id).unwrap();

        let inserted_assignment_id = assignment_persistence.create(&new_assignment).unwrap();

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
        dotenv().ok();

        let new_assignment = Assignment {
            id: "assignment-under-many-test".to_owned(),
            deployment_id: "deployment-fixture".to_owned(),
            host_id: "host-fixture".to_owned(),
        };

        let assignment_persistence = AssignmentRelationalPersistence::default();

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
