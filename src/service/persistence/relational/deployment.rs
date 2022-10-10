use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

use crate::models::Deployment;
use crate::persistence::{DeploymentPersistence, Persistence};

#[derive(Debug)]
pub struct DeploymentRelationalPersistence {
    pub db: Arc<PgPool>,
}

#[async_trait]
impl Persistence<Deployment> for DeploymentRelationalPersistence {
    #[tracing::instrument(name = "relational::deployment::create")]
    async fn upsert(&self, deployment: &Deployment) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO deployments
               (id, name, workload_id, target_id, template_id, host_count)
            VALUES
               ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
               name = $2,
               workload_id = $3,
               target_id = $4,
               template_id = $5,
               host_count = $6
            "#,
            deployment.id,
            deployment.name,
            deployment.workload_id,
            deployment.target_id,
            deployment.template_id,
            deployment.host_count
        )
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::deployment::delete")]
    async fn delete(&self, id: &str) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM deployments WHERE id = $1
            "#,
            id
        )
        .bind(id)
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::deployment::list")]
    async fn list(&self) -> anyhow::Result<Vec<Deployment>> {
        let rows = sqlx::query_as!(
            Deployment,
            r#"
                SELECT * FROM deployments
            "#,
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows
            .into_iter()
            .map(Deployment::from)
            .collect::<Vec<Deployment>>();

        Ok(models)
    }

    #[tracing::instrument(name = "relational::deployment::get_by_id")]
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Deployment>> {
        let supply = sqlx::query_as!(Deployment, "SELECT * FROM deployments WHERE id = $1", id)
            .fetch_optional(&*self.db)
            .await?;

        Ok(supply)
    }
}

#[async_trait]
impl DeploymentPersistence for DeploymentRelationalPersistence {
    #[tracing::instrument(name = "relational::deployment::get_by_target_id")]
    async fn get_by_target_id(&self, target_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let rows = sqlx::query_as!(
            Deployment,
            r#"
                SELECT * FROM deployments WHERE target_id = $1
            "#,
            target_id
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows
            .into_iter()
            .map(Deployment::from)
            .collect::<Vec<Deployment>>();

        Ok(models)
    }

    #[tracing::instrument(name = "relational::deployment::get_by_template_id")]
    async fn get_by_template_id(&self, template_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let rows = sqlx::query_as!(
            Deployment,
            r#"
                SELECT * FROM deployments WHERE template_id = $1
            "#,
            template_id
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows
            .into_iter()
            .map(Deployment::from)
            .collect::<Vec<Deployment>>();

        Ok(models)
    }

    #[tracing::instrument(name = "relational::deployment::get_by_workload_id")]
    async fn get_by_workload_id(&self, workload_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let rows = sqlx::query_as!(
            Deployment,
            r#"
                SELECT * FROM deployments WHERE workload_id = $1
            "#,
            workload_id
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows
            .into_iter()
            .map(Deployment::from)
            .collect::<Vec<Deployment>>();

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use fabriq_core::test::{get_deployment_fixture, get_target_fixture, get_template_fixture};

    use super::*;
    use crate::models::Deployment;
    use crate::persistence::relational::tests::ensure_fixtures;

    #[tokio::test]
    async fn test_deployment_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let deployment_persistence = DeploymentRelationalPersistence { db };
        let new_deployment: Deployment =
            get_deployment_fixture(Some("create-deployment-fixture")).into();

        deployment_persistence
            .delete(&new_deployment.id)
            .await
            .unwrap();
        deployment_persistence
            .upsert(&new_deployment)
            .await
            .unwrap();

        let fetched_deployment = deployment_persistence
            .get_by_id(&new_deployment.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_deployment.id, new_deployment.id);

        let deleted_deployments = deployment_persistence
            .delete(&new_deployment.id)
            .await
            .unwrap();
        assert_eq!(deleted_deployments, 1);
    }

    #[tokio::test]
    async fn test_get_by_target_id() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let deployment_persistence = DeploymentRelationalPersistence { db };
        let target_fixture = get_target_fixture(None);

        let deployments_for_target = deployment_persistence
            .get_by_target_id(&target_fixture.id)
            .await
            .unwrap();

        assert!(!deployments_for_target.is_empty());
    }

    #[tokio::test]
    async fn test_get_by_template_id() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let deployment_persistence = DeploymentRelationalPersistence { db };
        let template_fixture = get_template_fixture(None);

        let deployments_for_template = deployment_persistence
            .get_by_template_id(&template_fixture.id)
            .await
            .unwrap();

        assert!(!deployments_for_template.is_empty());
    }
}
