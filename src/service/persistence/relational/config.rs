use akira_core::ConfigMessage;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;

use crate::persistence::{ConfigPersistence, Persistence};
use crate::schema::configs::table;
use crate::{models::Config, schema::configs::dsl::*};

#[derive(Default, Debug)]
pub struct ConfigRelationalPersistence {}

impl Persistence<Config> for ConfigRelationalPersistence {
    #[tracing::instrument(name = "relational::config::create")]
    fn create(&self, config: &Config) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        let changed = diesel::insert_into(table)
            .values(config)
            .on_conflict(id)
            .do_update()
            .set(value.eq(config.value.clone()))
            .execute(&connection)?;

        Ok(changed)
    }

    #[tracing::instrument(name = "relational::config::create_many")]
    fn create_many(&self, models: &[Config]) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        let changed = diesel::insert_into(table)
            .values(models)
            .on_conflict(id)
            .do_update()
            .set(id.eq(excluded(id)))
            .execute(&connection)?;

        Ok(changed)
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

        let query_owning_model =
            ConfigMessage::make_owning_model("deployment", query_deployment_id)?;

        println!("query_owning_model: {:#?}", query_owning_model);

        let results = configs
            .filter(owning_model.eq(query_owning_model))
            .load::<Config>(&connection)?;

        Ok(results)
    }

    fn get_by_template_id(&self, query_template_id: &str) -> anyhow::Result<Vec<Config>> {
        let connection = crate::db::get_connection()?;

        let query_owning_model = ConfigMessage::make_owning_model("template", query_template_id)?;

        let results = configs
            .filter(owning_model.eq(query_owning_model))
            .load::<Config>(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::config::get_by_workload_id")]
    fn get_by_workload_id(&self, query_workload_id: &str) -> anyhow::Result<Vec<Config>> {
        let connection = crate::db::get_connection()?;

        let query_owning_model = ConfigMessage::make_owning_model("workload", query_workload_id)?;

        let results = configs
            .filter(owning_model.eq(query_owning_model))
            .load::<Config>(&connection)?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::{
        get_deployment_fixture, get_keyvalue_config_fixture, get_string_config_fixture,
        get_workload_fixture,
    };

    use super::*;
    use crate::models::Config;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let config: Config = get_string_config_fixture().into();
        let config_workload = get_workload_fixture(None);

        let config_persistence = ConfigRelationalPersistence::default();

        // delete config if it exists
        config_persistence.delete(&config.id).unwrap();
        let created_count = config_persistence.create(&config).unwrap();

        assert_eq!(created_count, 1);

        let fetched_config = config_persistence.get_by_id(&config.id).unwrap().unwrap();

        assert_eq!(fetched_config.id, config.id);

        let configs_for_workload = config_persistence
            .get_by_workload_id(&config_workload.id)
            .unwrap();

        assert_eq!(configs_for_workload.len(), 1);

        let deleted_configs = config_persistence.delete(&config.id).unwrap();

        assert_eq!(deleted_configs, 1);
    }

    #[test]
    fn test_create_delete_many() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let config_persistence = ConfigRelationalPersistence::default();
        let new_config: Config = get_keyvalue_config_fixture().into();
        let config_deployment = get_deployment_fixture(None);

        let created_configs = config_persistence
            .create_many(&[new_config.clone()])
            .unwrap();
        assert_eq!(created_configs, 1);

        let configs_for_deployment = config_persistence
            .get_by_deployment_id(&config_deployment.id)
            .unwrap();
        assert_eq!(configs_for_deployment.len(), 1);

        let deleted_configs = config_persistence.delete_many(&[&new_config.id]).unwrap();
        assert_eq!(deleted_configs, 1);
    }
}
