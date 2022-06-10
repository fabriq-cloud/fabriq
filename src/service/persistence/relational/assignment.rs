use async_trait::async_trait;
use diesel::prelude::*;

use crate::persistence::AssignmentPersistence;
use crate::schema::assignments::table;
use crate::{models::Assignment, schema::assignments, schema::assignments::dsl::*};

#[derive(Default)]
pub struct AssignmentRelationalPersistence {}

#[async_trait]
impl AssignmentPersistence for AssignmentRelationalPersistence {
    async fn create(&self, assignment: Assignment) -> anyhow::Result<String> {
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

    async fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(assignments.filter(id.eq(model_id))).execute(&connection)?)
    }

    async fn get_by_id(&self, assignment_id: &str) -> anyhow::Result<Option<Assignment>> {
        let connection = crate::db::get_connection()?;

        let results = assignments
            .filter(id.eq(assignment_id))
            .load::<Assignment>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }

    async fn get_by_deployment_id(&self, deploy_id: &str) -> anyhow::Result<Vec<Assignment>> {
        let connection = crate::db::get_connection()?;

        let results = assignments
            .filter(deployment_id.eq(deploy_id))
            .load::<Assignment>(&connection)?;

        Ok(results)
    }

    async fn list(&self) -> anyhow::Result<Vec<Assignment>> {
        let connection = crate::db::get_connection()?;

        let results = assignments.load::<Assignment>(&connection).unwrap();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;
    use crate::models::Assignment;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();
        crate::persistence::relational::ensure_fixtures().await;

        let new_assignment = Assignment {
            id: "assignment-under-test".to_owned(),
            deployment_id: "deployment-fixture".to_owned(),
            host_id: "host-fixture".to_owned(),
        };

        let assignment_persistence = AssignmentRelationalPersistence::default();

        // delete assignment if it exists
        let _ = assignment_persistence
            .delete(&new_assignment.id)
            .await
            .unwrap();

        let inserted_assignment_id = assignment_persistence
            .create(new_assignment.clone())
            .await
            .unwrap();

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
