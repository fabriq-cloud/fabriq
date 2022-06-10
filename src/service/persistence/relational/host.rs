use async_trait::async_trait;
use diesel::prelude::*;

use crate::{
    diesel::RunQueryDsl,
    models::{Host, Target},
    persistence::HostPersistence,
    schema::hosts,
    schema::hosts::dsl::*,
};

#[derive(Default)]
pub struct HostRelationalPersistence {}

#[async_trait]
impl HostPersistence for HostRelationalPersistence {
    async fn create(&self, host: Host) -> anyhow::Result<String> {
        let conn = crate::db::get_connection()?;

        let results: Vec<String> = diesel::insert_into(hosts::table)
            .values(host)
            .returning(hosts::id)
            .get_results(&conn)?;

        match results.first() {
            Some(host_id) => Ok(host_id.clone()),
            None => Err(anyhow::anyhow!("Couldn't find created host id returned")),
        }
    }

    async fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(hosts.filter(id.eq(model_id))).execute(&connection)?)
    }

    async fn list(&self) -> anyhow::Result<Vec<Host>> {
        let connection = crate::db::get_connection()?;

        Ok(hosts.load::<Host>(&connection)?)
    }

    async fn get_by_id(&self, host_id: &str) -> anyhow::Result<Option<Host>> {
        let connection = crate::db::get_connection()?;

        let results = hosts.filter(id.eq(host_id)).load::<Host>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }

    async fn get_matching_target(&self, target: &Target) -> anyhow::Result<Vec<Host>> {
        let connection = crate::db::get_connection()?;

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

        let _ = host_persistence.delete(&new_host.id).await.unwrap();

        let inserted_host_id = host_persistence.create(new_host.clone()).await.unwrap();

        let fetched_host = host_persistence
            .get_by_id(&inserted_host_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_host.id, new_host.id);

        let deleted_hosts = host_persistence.delete(&inserted_host_id).await.unwrap();
        assert_eq!(deleted_hosts, 1);
    }

    #[tokio::test]
    async fn test_get_by_id() {
        dotenv().ok();
        crate::persistence::relational::ensure_fixtures().await;

        let host_persistence = HostRelationalPersistence::default();

        let fetched_host = host_persistence
            .get_by_id("host-fixture")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_host.id, "host-fixture");
    }

    #[tokio::test]
    async fn test_get_matching_target() {
        dotenv().ok();
        crate::persistence::relational::ensure_fixtures().await;

        let host_persistence = HostRelationalPersistence::default();

        let matching_target = Target {
            id: "target-eastus2".to_owned(),
            labels: vec!["region:eastus2".to_string()],
        };

        let matching_hosts = host_persistence
            .get_matching_target(&matching_target)
            .await
            .unwrap();

        assert_eq!(matching_hosts.len(), 1);

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
