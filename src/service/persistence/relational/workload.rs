use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::models::Workload;
use crate::persistence::{Persistence, WorkloadPersistence};

#[derive(Debug)]
pub struct WorkloadRelationalPersistence {
    pub db: Arc<PgPool>,
}

#[async_trait]
impl Persistence<Workload> for WorkloadRelationalPersistence {
    #[tracing::instrument(name = "relational::workload::create")]
    async fn upsert(&self, workload: &Workload) -> anyhow::Result<u64> {
        /*
        pub id: String,
        pub name: String,
        pub team_id: String,
        pub template_id: String,
        */

        let result = sqlx::query!(
            r#"
            INSERT INTO workloads
               (id, name, team_id, template_id)
            VALUES
               ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
               name = $2,
               team_id = $3,
               template_id = $4
            "#,
            workload.id,
            workload.name,
            workload.team_id,
            workload.template_id,
        )
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::workload::delete")]
    async fn delete(&self, id: &str) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM workloads WHERE id = $1
            "#,
            id
        )
        .bind(id)
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::workload::list")]
    async fn list(&self) -> anyhow::Result<Vec<Workload>> {
        let rows = sqlx::query_as!(
            Workload,
            r#"
                SELECT * FROM workloads
            "#,
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows
            .into_iter()
            .map(Workload::from)
            .collect::<Vec<Workload>>();

        Ok(models)
    }

    #[tracing::instrument(name = "relational::workload::get_by_id")]
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Workload>> {
        let supply = sqlx::query_as!(Workload, "SELECT * FROM workloads WHERE id = $1", id)
            .fetch_optional(&*self.db)
            .await?;

        Ok(supply)
    }
}

#[async_trait]
impl WorkloadPersistence for WorkloadRelationalPersistence {
    #[tracing::instrument(name = "relational::workload::get_by_template_id")]
    async fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Workload>> {
        let rows = sqlx::query_as!(
            Workload,
            r#"
                SELECT * FROM workloads WHERE template_id = $1
            "#,
            template_id
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows
            .into_iter()
            .map(Workload::from)
            .collect::<Vec<Workload>>();

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::get_workload_fixture;

    use super::*;
    use crate::models::Workload;
    use crate::persistence::relational::tests::ensure_fixtures;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let workload_persistence = WorkloadRelationalPersistence { db };
        let workload: Workload = get_workload_fixture(Some("relational-workload-create")).into();

        workload_persistence.delete(&workload.id).await.unwrap();

        let created_count = workload_persistence.upsert(&workload).await.unwrap();

        assert_eq!(created_count, 1);

        let fetched_workload = workload_persistence
            .get_by_id(&workload.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_workload.id, workload.id);

        let deleted_workloads = workload_persistence.delete(&workload.id).await.unwrap();

        assert_eq!(deleted_workloads, 1);
    }
}
