use diesel::prelude::*;

use crate::{
    diesel::RunQueryDsl,
    models::{Host, Target},
    persistence::{HostPersistence, Persistence},
    schema::hosts,
    schema::hosts::dsl::*,
    schema::hosts::table,
};

#[derive(Default)]
pub struct HostRelationalPersistence {}

impl Persistence<Host> for HostRelationalPersistence {
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

    fn create_many(&self, models: &[Host]) -> anyhow::Result<Vec<String>> {
        let connection = crate::db::get_connection()?;

        let results = diesel::insert_into(table)
            .values(models)
            .returning(hosts::id)
            .get_results(&connection)?;

        Ok(results)
    }

    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(hosts.filter(id.eq(model_id))).execute(&connection)?)
    }

    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, model_id) in model_ids.iter().enumerate() {
            self.delete(model_id)?;
        }

        Ok(model_ids.len())
    }

    fn list(&self) -> anyhow::Result<Vec<Host>> {
        let connection = crate::db::get_connection()?;

        Ok(hosts.load::<Host>(&connection)?)
    }

    fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Host>> {
        let connection = crate::db::get_connection()?;

        let results = hosts.filter(id.eq(host_id)).load::<Host>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }
}

impl HostPersistence for HostRelationalPersistence {
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
    use dotenv::dotenv;

    use super::*;

    #[tokio::test]
    async fn test_create_delete() {
        dotenv().ok();

        let new_host = Host {
            id: "host-under-test".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
        };

        let host_persistence = HostRelationalPersistence::default();

        let _ = host_persistence.delete(&new_host.id).unwrap();

        let inserted_host_id = host_persistence.create(&new_host).unwrap();

        let fetched_host = host_persistence
            .get_by_id(&inserted_host_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_host.id, new_host.id);

        let deleted_hosts = host_persistence.delete(&inserted_host_id).unwrap();
        assert_eq!(deleted_hosts, 1);
    }

    #[test]
    fn test_get_by_id() {
        dotenv().ok();
        crate::persistence::relational::ensure_fixtures();

        let host_persistence = HostRelationalPersistence::default();

        let fetched_host = host_persistence.get_by_id("host-fixture").unwrap().unwrap();
        assert_eq!(fetched_host.id, "host-fixture");
    }

    #[test]
    fn test_get_matching_target() {
        dotenv().ok();
        crate::persistence::relational::ensure_fixtures();

        let host_persistence = HostRelationalPersistence::default();

        let matching_target = Target {
            id: "target-eastus2".to_owned(),
            labels: vec!["region:eastus2".to_string()],
        };

        let matching_hosts = host_persistence
            .get_matching_target(&matching_target)
            .unwrap();

        assert_eq!(matching_hosts.len(), 1);

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
        dotenv().ok();

        let new_host = Host {
            id: "host-under-many-test".to_owned(),
            labels: vec!["cloud:aws".to_owned(), "region:westus2".to_owned()],
        };

        let host_persistence = HostRelationalPersistence::default();

        let inserted_host_ids = host_persistence.create_many(&[new_host.clone()]).unwrap();
        assert_eq!(inserted_host_ids.len(), 1);
        assert_eq!(inserted_host_ids[0], new_host.id);

        let deleted_hosts = host_persistence.delete_many(&[&new_host.id]).unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
