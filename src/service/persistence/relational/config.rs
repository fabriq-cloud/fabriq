use diesel::prelude::*;

use crate::persistence::{ConfigPersistence, Persistence};
use crate::schema::configs::table;
use crate::{models::Config, schema::configs, schema::configs::dsl::*};

#[derive(Default, Debug)]
pub struct ConfigRelationalPersistence {}

impl Persistence<Config> for ConfigRelationalPersistence {
    #[tracing::instrument(name = "relational::config::create")]
    fn create(&self, config: &Config) -> anyhow::Result<String> {
        let connection = crate::db::get_connection()?;

        let results: Vec<String> = diesel::insert_into(table)
            .values(config)
            .returning(configs::id)
            .get_results(&connection)?;

        match results.first() {
            Some(host_id) => Ok(host_id.clone()),
            None => Err(anyhow::anyhow!("Couldn't find created host id returned")),
        }
    }

    #[tracing::instrument(name = "relational::config::create_many")]
    fn create_many(&self, models: &[Config]) -> anyhow::Result<Vec<String>> {
        let connection = crate::db::get_connection()?;

        let results = diesel::insert_into(table)
            .values(models)
            .returning(configs::id)
            .get_results(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::config::delete")]
    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(configs.filter(id.eq(model_id))).execute(&connection)?)
    }

    #[tracing::instrument(name = "relational::config::delete_many")]
    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, model_id) in model_ids.iter().enumerate() {
            self.delete(model_id)?;
        }

        Ok(model_ids.len())
    }

    #[tracing::instrument(name = "relational::config::list")]
    fn list(&self) -> anyhow::Result<Vec<Config>> {
        let connection = crate::db::get_connection()?;

        let results = configs.load::<Config>(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::config::get_by_id")]
    fn get_by_id(&self, config_id: &str) -> anyhow::Result<Option<Config>> {
        let connection = crate::db::get_connection()?;

        let results = configs
            .filter(id.eq(config_id))
            .load::<Config>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }
}

impl ConfigPersistence for ConfigRelationalPersistence {
    #[tracing::instrument(name = "relational::config::get_by_deployment_id")]
    fn get_by_deployment_id(&self, query_deployment_id: &str) -> anyhow::Result<Vec<Config>> {
        let connection = crate::db::get_connection()?;

        let query_owning_model = Config::make_owning_model("deployment", query_deployment_id);

        let results = configs
            .filter(owning_model.eq(query_owning_model))
            .load::<Config>(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::config::get_by_workload_id")]
    fn get_by_workload_id(&self, query_workload_id: &str) -> anyhow::Result<Vec<Config>> {
        let connection = crate::db::get_connection()?;

        let query_owning_model = Config::make_owning_model("workload", query_workload_id);

        let results = configs
            .filter(owning_model.eq(query_owning_model))
            .load::<Config>(&connection)?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Config;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let new_config = Config {
            id: "config-persist-single-under-test".to_owned(),

            owning_model: "workload:workload-fixture".to_owned(),

            key: "sample-key".to_owned(),
            value: "sample-value".to_owned(),
        };

        let config_persistence = ConfigRelationalPersistence::default();

        // delete config if it exists
        let _ = config_persistence.delete(&new_config.id).unwrap();
        let inserted_config_id = config_persistence.create(&new_config).unwrap();

        let fetched_config = config_persistence
            .get_by_id(&inserted_config_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_config.id, new_config.id);

        let configs_for_workload = config_persistence
            .get_by_workload_id("workload-fixture")
            .unwrap();
        assert_eq!(configs_for_workload.len(), 1);

        let deleted_configs = config_persistence.delete(&inserted_config_id).unwrap();
        assert_eq!(deleted_configs, 1);
    }

    #[test]
    fn test_create_delete_many() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let new_config = Config {
            id: "config-persist-many-under-test".to_owned(),

            owning_model: "deployment:deployment-fixture".to_owned(),

            key: "sample-key".to_owned(),
            value: "sample-value".to_owned(),
        };

        let config_persistence = ConfigRelationalPersistence::default();

        let inserted_config_ids = config_persistence
            .create_many(&[new_config.clone()])
            .unwrap();
        assert_eq!(inserted_config_ids.len(), 1);
        assert_eq!(inserted_config_ids[0], new_config.id);

        let configs_for_deployment = config_persistence
            .get_by_deployment_id("deployment-fixture")
            .unwrap();
        assert_eq!(configs_for_deployment.len(), 1);

        let deleted_configs = config_persistence.delete_many(&[&new_config.id]).unwrap();
        assert_eq!(deleted_configs, 1);
    }
}
