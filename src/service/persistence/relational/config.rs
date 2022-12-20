use async_trait::async_trait;
use fabriq_core::ConfigMessage;
use sqlx::PgPool;
use std::sync::Arc;

use crate::models::Config;
use crate::persistence::{ConfigPersistence, Persistence};

#[derive(Debug)]
pub struct ConfigRelationalPersistence {
    pub db: Arc<PgPool>,
}

#[async_trait]
impl Persistence<Config> for ConfigRelationalPersistence {
    #[tracing::instrument(name = "relational::config::upsert", skip_all)]
    async fn upsert(&self, config: &Config) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO configs
               (id, owning_model, key, value, value_type)
            VALUES
               ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
               owning_model = $2,
               key = $3,
               value = $4,
               value_type = $5
            "#,
            config.id,
            config.owning_model,
            config.key,
            config.value,
            config.value_type
        )
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::config::delete", skip_all)]
    async fn delete(&self, id: &str) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM configs WHERE id = $1
            "#,
            id
        )
        .bind(id)
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::config::list", skip_all)]
    async fn list(&self) -> anyhow::Result<Vec<Config>> {
        let rows = sqlx::query_as!(
            Config,
            r#"
                SELECT * FROM configs
            "#,
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows.into_iter().map(Config::from).collect::<Vec<Config>>();

        Ok(models)
    }

    #[tracing::instrument(name = "relational::config::get_by_id", skip_all)]
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Config>> {
        let supply = sqlx::query_as!(Config, "SELECT * FROM configs WHERE id = $1", id)
            .fetch_optional(&*self.db)
            .await?;

        Ok(supply)
    }
}

impl ConfigRelationalPersistence {
    #[tracing::instrument(name = "relational::config::get_owning_model", skip_all)]
    async fn get_owning_model(&self, owning_model: &str) -> anyhow::Result<Vec<Config>> {
        let rows = sqlx::query_as!(
            Config,
            r#"
                SELECT * FROM configs WHERE owning_model = $1
            "#,
            owning_model
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows.into_iter().map(Config::from).collect::<Vec<Config>>();

        Ok(models)
    }
}

#[async_trait]
impl ConfigPersistence for ConfigRelationalPersistence {
    #[tracing::instrument(name = "relational::config::get_by_deployment_id", skip_all)]
    async fn get_by_deployment_id(&self, query_deployment_id: &str) -> anyhow::Result<Vec<Config>> {
        let query_owning_model =
            ConfigMessage::make_owning_model("deployment", query_deployment_id)?;

        self.get_owning_model(&query_owning_model).await
    }

    #[tracing::instrument(name = "relational::config::get_by_template_id", skip_all)]
    async fn get_by_template_id(&self, query_template_id: &str) -> anyhow::Result<Vec<Config>> {
        let query_owning_model = ConfigMessage::make_owning_model("template", query_template_id)?;

        self.get_owning_model(&query_owning_model).await
    }

    #[tracing::instrument(name = "relational::config::get_by_workload_id", skip_all)]
    async fn get_by_workload_id(&self, query_workload_id: &str) -> anyhow::Result<Vec<Config>> {
        let query_owning_model = ConfigMessage::make_owning_model("workload", query_workload_id)?;

        self.get_owning_model(&query_owning_model).await
    }
}

#[cfg(test)]
mod tests {
    use fabriq_core::test::{get_string_config_fixture, get_workload_fixture};

    use super::*;
    use crate::{models::Config, persistence::relational::tests::ensure_fixtures};

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let config: Config = get_string_config_fixture().into();
        let config_workload = get_workload_fixture(None);

        let config_persistence = ConfigRelationalPersistence { db };

        // delete config if it exists
        config_persistence.delete(&config.id).await.unwrap();
        let created_count = config_persistence.upsert(&config).await.unwrap();

        assert_eq!(created_count, 1);

        let fetched_config = config_persistence
            .get_by_id(&config.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_config.id, config.id);

        let configs_for_workload = config_persistence
            .get_by_workload_id(&config_workload.id)
            .await
            .unwrap();

        assert_eq!(configs_for_workload.len(), 1);

        let deleted_configs = config_persistence.delete(&config.id).await.unwrap();

        assert_eq!(deleted_configs, 1);
    }
}
