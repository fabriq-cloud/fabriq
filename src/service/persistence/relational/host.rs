use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::{
    models::{Host, Target},
    persistence::{HostPersistence, Persistence},
};

#[derive(Debug)]
pub struct HostRelationalPersistence {
    pub db: Arc<PgPool>,
}

#[async_trait]
impl Persistence<Host> for HostRelationalPersistence {
    #[tracing::instrument(name = "relational::host::create")]
    async fn upsert(&self, host: &Host) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO hosts
               (id, labels)
            VALUES
               ($1, $2)
            ON CONFLICT (id) DO UPDATE SET
               labels = $2
            "#,
            host.id,
            &host.labels
        )
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::host::delete")]
    async fn delete(&self, id: &str) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM hosts WHERE id = $1
            "#,
            id
        )
        .bind(id)
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::host::list")]
    async fn list(&self) -> anyhow::Result<Vec<Host>> {
        let rows = sqlx::query_as!(
            Host,
            r#"
                SELECT * FROM hosts
            "#,
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows.into_iter().map(Host::from).collect::<Vec<Host>>();

        Ok(models)
    }

    #[tracing::instrument(name = "relational::host::get_by_id")]
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Host>> {
        let supply = sqlx::query_as!(Host, "SELECT * FROM hosts WHERE id = $1", id)
            .fetch_optional(&*self.db)
            .await?;

        Ok(supply)
    }
}

#[async_trait]
impl HostPersistence for HostRelationalPersistence {
    #[tracing::instrument]
    async fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>> {
        // $1 <@ labels matches the set of hosts that have target.labels
        let rows = sqlx::query_as!(
            Host,
            r#"
                SELECT * FROM hosts WHERE $1 <@ labels
            "#,
            &target.labels
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows.into_iter().map(Host::from).collect::<Vec<Host>>();

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::{get_host_fixture, get_target_fixture};

    use super::*;
    use crate::persistence::relational::tests::ensure_fixtures;

    #[tokio::test]
    async fn test_create_delete() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let host_persistence = HostRelationalPersistence { db };
        let host: Host = get_host_fixture(Some("host-create")).into();

        host_persistence.delete(&host.id).await.unwrap();

        let changed_count = host_persistence.upsert(&host).await.unwrap();
        assert_eq!(changed_count, 1);

        let fetched_host = host_persistence.get_by_id(&host.id).await.unwrap().unwrap();
        assert_eq!(fetched_host.id, host.id);

        let deleted_hosts = host_persistence.delete(&host.id).await.unwrap();
        assert_eq!(deleted_hosts, 1);
    }

    #[tokio::test]
    async fn test_get_by_id() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let host_persistence = HostRelationalPersistence { db };
        let host: Host = get_host_fixture(None).into();

        let fetched_host = host_persistence.get_by_id(&host.id).await.unwrap().unwrap();
        assert_eq!(fetched_host.id, host.id);
    }

    #[tokio::test]
    async fn test_get_matching_target() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let host_persistence = HostRelationalPersistence { db };

        let matching_target: Target = get_target_fixture(None).into();

        let matching_hosts = host_persistence
            .get_matching_target(&matching_target)
            .await
            .unwrap();

        assert!(!matching_hosts.is_empty());

        let non_matching_target = Target {
            id: "target-hawaii".to_owned(),
            labels: vec!["region:hawaii5".to_string()],
        };

        let non_matching_hosts = host_persistence
            .get_matching_target(&non_matching_target)
            .await
            .unwrap();

        assert!(non_matching_hosts.is_empty());
    }
}
