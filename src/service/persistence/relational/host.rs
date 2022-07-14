use diesel::prelude::*;

use crate::{
    diesel::RunQueryDsl,
    models::{Host, Target},
    persistence::{HostPersistence, Persistence},
    schema::hosts,
    schema::hosts::dsl::*,
    schema::hosts::table,
};

#[derive(Default, Debug)]
pub struct HostRelationalPersistence {}

impl Persistence<Host> for HostRelationalPersistence {
    #[tracing::instrument(name = "relational::host::create")]
    fn create(&self, host: &Host) -> anyhow::Result<String> {
        let conn = crate::db::get_connection()?;

        let results: Vec<String> = diesel::insert_into(table)
            .values(host)
            .returning(hosts::id)
            .get_results(&conn)?;

        match results.first() {
            Some(host_id) => Ok(host_id.clone()),
            None => Err(anyhow::anyhow!("Couldn't find created host id returned")),
        }
    }

    #[tracing::instrument(name = "relational::host::create_many")]
    fn create_many(&self, models: &[Host]) -> anyhow::Result<Vec<String>> {
        let connection = crate::db::get_connection()?;

        let results = diesel::insert_into(table)
            .values(models)
            .returning(hosts::id)
            .get_results(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::host::delete")]
    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(hosts.filter(id.eq(model_id))).execute(&connection)?)
    }

    #[tracing::instrument(name = "relational::host::delete_many")]
    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, model_id) in model_ids.iter().enumerate() {
            self.delete(model_id)?;
        }

        Ok(model_ids.len())
    }

    #[tracing::instrument(name = "relational::host::list")]
    fn list(&self) -> anyhow::Result<Vec<Host>> {
        let connection = crate::db::get_connection()?;

        Ok(hosts.load::<Host>(&connection)?)
    }

    #[tracing::instrument(name = "relational::host::get_by_id")]
    fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Host>> {
        let connection = crate::db::get_connection()?;

        let results = hosts.filter(id.eq(host_id)).load::<Host>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }
}

impl HostPersistence for HostRelationalPersistence {
    #[tracing::instrument]
    fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>> {
        let connection = crate::db::get_connection()?;

        // TODO: Can imagine labels of hosts being indexed and using a more efficient query
        let all_hosts = hosts.load::<Host>(&connection)?;

        let matching_hosts = all_hosts
            .into_iter()
            .filter(|host| {
                for label in target.labels.iter() {
                    if !host.labels.contains(label) {
                        return false;
                    }
                }

                true
            })
            .collect();

        Ok(matching_hosts)
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::{get_host_fixture, get_target_fixture};

    use super::*;

    #[tokio::test]
    async fn test_create_delete() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let host_persistence = HostRelationalPersistence::default();
        let host: Host = get_host_fixture(Some("host-create")).into();

        host_persistence.delete(&host.id).unwrap();

        let inserted_host_id = host_persistence.create(&host).unwrap();

        let fetched_host = host_persistence
            .get_by_id(&inserted_host_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_host.id, host.id);

        let deleted_hosts = host_persistence.delete(&inserted_host_id).unwrap();
        assert_eq!(deleted_hosts, 1);
    }

    #[test]
    fn test_get_by_id() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let host_persistence = HostRelationalPersistence::default();
        let host: Host = get_host_fixture(None).into();

        let fetched_host = host_persistence.get_by_id(&host.id).unwrap().unwrap();
        assert_eq!(fetched_host.id, host.id);
    }

    #[test]
    fn test_get_matching_target() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let host_persistence = HostRelationalPersistence::default();

        let matching_target: Target = get_target_fixture(None).into();

        let matching_hosts = host_persistence
            .get_matching_target(&matching_target)
            .unwrap();

        assert!(!matching_hosts.is_empty());

        let non_matching_target = Target {
            id: "target-hawaii".to_owned(),
            labels: vec!["region:hawaii5".to_string()],
        };

        let non_matching_hosts = host_persistence
            .get_matching_target(&non_matching_target)
            .unwrap();

        assert!(non_matching_hosts.is_empty());
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();

        let host_persistence = HostRelationalPersistence::default();
        let host: Host = get_host_fixture(Some("relational-host-create-many")).into();

        host_persistence.delete(&host.id).unwrap();

        let inserted_host_ids = host_persistence.create_many(&[host.clone()]).unwrap();
        assert_eq!(inserted_host_ids.len(), 1);
        assert_eq!(inserted_host_ids[0], host.id);

        let deleted_hosts = host_persistence.delete_many(&[&host.id]).unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
