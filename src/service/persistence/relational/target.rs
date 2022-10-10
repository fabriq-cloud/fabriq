use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::models::{Host, Target};
use crate::persistence::{Persistence, TargetPersistence};

#[derive(Debug)]
pub struct TargetRelationalPersistence {
    pub db: Arc<PgPool>,
}

#[async_trait]
impl Persistence<Target> for TargetRelationalPersistence {
    #[tracing::instrument(name = "relational::target::create")]
    async fn upsert(&self, target: &Target) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO targets
               (id, labels)
            VALUES
               ($1, $2)
            ON CONFLICT (id) DO UPDATE SET
               labels = $2
            "#,
            target.id,
            &target.labels,
        )
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::target::delete")]
    async fn delete(&self, id: &str) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM targets WHERE id = $1
            "#,
            id
        )
        .bind(id)
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::target::list")]
    async fn list(&self) -> anyhow::Result<Vec<Target>> {
        let rows = sqlx::query_as!(
            Target,
            r#"
                SELECT * FROM targets
            "#,
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows.into_iter().map(Target::from).collect::<Vec<Target>>();

        Ok(models)
    }

    #[tracing::instrument(name = "relational::target::get_by_id")]
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Target>> {
        let supply = sqlx::query_as!(Target, "SELECT * FROM targets WHERE id = $1", id)
            .fetch_optional(&*self.db)
            .await?;

        Ok(supply)
    }
}

#[async_trait]
impl TargetPersistence for TargetRelationalPersistence {
    #[tracing::instrument]
    async fn get_matching_host(&self, host: &Host) -> anyhow::Result<Vec<Target>> {
        // $1 <@ labels matches the set of hosts that have target.labels
        let rows = sqlx::query_as!(
            Target,
            r#"
                SELECT * FROM targets WHERE $1 <@ labels
            "#,
            &host.labels
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows.into_iter().map(Target::from).collect::<Vec<Target>>();

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use fabriq_core::test::get_target_fixture;

    use super::*;
    use crate::models::Target;
    use crate::persistence::relational::tests::ensure_fixtures;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let target_persistence = TargetRelationalPersistence { db };
        let target: Target = get_target_fixture(Some("target-create")).into();

        // delete target if it exists
        target_persistence.delete(&target.id).await.unwrap();

        let created_count = target_persistence.upsert(&target).await.unwrap();

        assert_eq!(created_count, 1);

        let fetched_target = target_persistence
            .get_by_id(&target.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_target.id, target.id);

        let deleted_targets = target_persistence.delete(&target.id).await.unwrap();
        assert_eq!(deleted_targets, 1);
    }
}
