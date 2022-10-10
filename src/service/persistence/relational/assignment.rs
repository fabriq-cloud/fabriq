use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

use crate::models::Assignment;
use crate::persistence::{AssignmentPersistence, Persistence};

#[derive(Debug)]
pub struct AssignmentRelationalPersistence {
    pub db: Arc<PgPool>,
}

#[async_trait]
impl Persistence<Assignment> for AssignmentRelationalPersistence {
    #[tracing::instrument(name = "relational::assignment::create")]
    async fn upsert(&self, assignment: &Assignment) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO assignments
               (id, deployment_id, host_id)
            VALUES
               ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET
               deployment_id = $2,
               host_id = $3
            "#,
            assignment.id,
            assignment.deployment_id,
            assignment.host_id
        )
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::assignment::delete")]
    async fn delete(&self, id: &str) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM assignments WHERE id = $1
            "#,
            id
        )
        .bind(id)
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::assignment::get_by_id")]
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Assignment>> {
        let supply = sqlx::query_as!(Assignment, "SELECT * FROM assignments WHERE id = $1", id)
            .fetch_optional(&*self.db)
            .await?;

        Ok(supply)
    }

    #[tracing::instrument(name = "relational::assignment::list")]
    async fn list(&self) -> anyhow::Result<Vec<Assignment>> {
        let rows = sqlx::query_as!(
            Assignment,
            r#"
                SELECT * FROM assignments
            "#,
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows
            .into_iter()
            .map(Assignment::from)
            .collect::<Vec<Assignment>>();

        Ok(models)
    }
}

#[async_trait]
impl AssignmentPersistence for AssignmentRelationalPersistence {
    async fn get_by_deployment_id(&self, deployment_id: &str) -> anyhow::Result<Vec<Assignment>> {
        let rows = sqlx::query_as!(
            Assignment,
            r#"
                SELECT * FROM assignments WHERE deployment_id = $1
            "#,
            deployment_id
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows
            .into_iter()
            .map(Assignment::from)
            .collect::<Vec<Assignment>>();

        Ok(models)
    }
}
#[cfg(test)]
mod tests {
    use akira_core::test::get_assignment_fixture;

    use super::*;
    use crate::models::Assignment;
    use crate::persistence::relational::tests::ensure_fixtures;

    #[tokio::test]
    async fn test_assignment_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let assignment_persistence = AssignmentRelationalPersistence { db };
        let assignment: Assignment = get_assignment_fixture(Some("assignment-create")).into();

        // delete assignment if it exists
        assignment_persistence.delete(&assignment.id).await.unwrap();

        let created_count = assignment_persistence.upsert(&assignment).await.unwrap();
        assert_eq!(created_count, 1);

        let fetched_assignment = assignment_persistence
            .get_by_id(&assignment.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_assignment.id, assignment.id);

        let deployment_assignments = assignment_persistence
            .get_by_deployment_id(&assignment.deployment_id)
            .await
            .unwrap();

        assert!(!deployment_assignments.is_empty());

        let deleted_assignments = assignment_persistence.delete(&assignment.id).await.unwrap();
        assert_eq!(deleted_assignments, 1);
    }
}
