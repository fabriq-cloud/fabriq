use diesel::prelude::*;

use crate::persistence::{AssignmentPersistence, Persistence};
use crate::schema::assignments::table;
use crate::{models::Assignment, schema::assignments, schema::assignments::dsl::*};

#[derive(Default, Debug)]
pub struct AssignmentRelationalPersistence {}

impl Persistence<Assignment> for AssignmentRelationalPersistence {
    #[tracing::instrument(name = "relational::assignment::create")]
    fn create(&self, assignment: &Assignment) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        let changed = diesel::insert_into(table)
            .values(assignment)
            .on_conflict(id)
            .do_nothing()
            .execute(&connection)?;

        Ok(changed)
    }

    #[tracing::instrument(name = "relational::assignment::create_many")]
    fn create_many(&self, models: &[Assignment]) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        let results = diesel::insert_into(table)
            .values(models)
            .returning(assignments::id)
            .on_conflict_do_nothing()
            .execute(&connection)?;

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
        let assignment: Assignment = get_assignment_fixture(Some("assignment-create")).into();

        // delete assignment if it exists
        assignment_persistence.delete(&assignment.id).unwrap();

        let created_count = assignment_persistence.create(&assignment).unwrap();
        assert_eq!(created_count, 1);

        let fetched_assignment = assignment_persistence
            .get_by_id(&assignment.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_assignment.id, assignment.id);

        let deployment_assignments = assignment_persistence
            .get_by_deployment_id(&assignment.deployment_id)
            .unwrap();

        assert!(!deployment_assignments.is_empty());

        let deleted_assignments = assignment_persistence.delete(&assignment.id).unwrap();
        assert_eq!(deleted_assignments, 1);
    }

    #[test]
    fn test_assigment_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let assignment_persistence = AssignmentRelationalPersistence::default();
        let new_assignment: Assignment =
            get_assignment_fixture(Some("assignment-create-many")).into();

        let created_count = assignment_persistence
            .create_many(&[new_assignment.clone()])
            .unwrap();
        assert_eq!(created_count, 1);

        let deleted_count = assignment_persistence
            .delete_many(&[&new_assignment.id])
            .unwrap();
        assert_eq!(deleted_count, 1);
    }
}
