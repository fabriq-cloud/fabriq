use diesel::prelude::*;

use crate::persistence::{AssignmentPersistence, Persistence};
use crate::schema::assignments::table;
use crate::{models::Assignment, schema::assignments, schema::assignments::dsl::*};

#[derive(Default, Debug)]
pub struct AssignmentRelationalPersistence {}

impl Persistence<Assignment> for AssignmentRelationalPersistence {
    #[tracing::instrument(name = "relational::assignment::create")]
    fn create(&self, assignment: &Assignment) -> anyhow::Result<String> {
        let connection = crate::db::get_connection()?;

        let results: Vec<String> = diesel::insert_into(table)
            .values(assignment)
            .returning(assignments::id)
            .on_conflict_do_nothing()
            .get_results(&connection)?;

        match results.first() {
            Some(assignment_id) => Ok(assignment_id.clone()),
            None => Err(anyhow::anyhow!(
                "Couldn't find created assignment id returned"
            )),
        }
    }

    #[tracing::instrument(name = "relational::assignment::create_many")]
    fn create_many(&self, models: &[Assignment]) -> anyhow::Result<Vec<String>> {
        let connection = crate::db::get_connection()?;

        let results = diesel::insert_into(table)
            .values(models)
            .returning(assignments::id)
            .on_conflict_do_nothing()
            .get_results(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::assignment::delete")]
    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(assignments.filter(id.eq(model_id))).execute(&connection)?)
    }

    #[tracing::instrument(name = "relational::assignment::delete_many")]
    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, model_id) in model_ids.iter().enumerate() {
            self.delete(model_id)?;
        }

        Ok(model_ids.len())
    }

    #[tracing::instrument(name = "relational::assignment::get_by_id")]
    fn get_by_id(&self, assignment_id: &str) -> anyhow::Result<Option<Assignment>> {
        let connection = crate::db::get_connection()?;

        let results = assignments
            .filter(id.eq(assignment_id))
            .load::<Assignment>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }

    #[tracing::instrument(name = "relational::assignment::list")]
    fn list(&self) -> anyhow::Result<Vec<Assignment>> {
        let connection = crate::db::get_connection()?;

        let results = assignments.load::<Assignment>(&connection)?;

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
    use akira_core::test::get_assignment_fixture;

    use super::*;
    use crate::models::Assignment;

    #[test]
    fn test_assignment_create_get_delete() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let assignment_persistence = AssignmentRelationalPersistence::default();
        let new_assignment: Assignment = get_assignment_fixture(Some("assignment-create")).into();

        // delete assignment if it exists
        assignment_persistence.delete(&new_assignment.id).unwrap();

        let inserted_assignment_id = assignment_persistence.create(&new_assignment).unwrap();

        let fetched_assignment = assignment_persistence
            .get_by_id(&inserted_assignment_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_assignment.id, new_assignment.id);

        let deployment_assignments = assignment_persistence
            .get_by_deployment_id(&new_assignment.deployment_id)
            .unwrap();

        assert!(!deployment_assignments.is_empty());

        let deleted_assignments = assignment_persistence
            .delete(&inserted_assignment_id)
            .unwrap();
        assert_eq!(deleted_assignments, 1);
    }

    #[test]
    fn test_assigment_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let assignment_persistence = AssignmentRelationalPersistence::default();
        let new_assignment: Assignment =
            get_assignment_fixture(Some("assignment-create-many")).into();

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
