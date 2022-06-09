use akira_core::Persistence;
use async_trait::async_trait;
use diesel::prelude::*;

use crate::{diesel::RunQueryDsl, models::Host, schema::hosts, schema::hosts::dsl::*};

#[derive(Default)]
pub struct HostRelationalPersistence {}

#[async_trait]
impl Persistence<Host, Host> for HostRelationalPersistence {
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
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        let new_host = Host {
            id: "host-under-test".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],

            cpu_capacity: 4000,
            memory_capacity: 24000,
        };

        let host_persistence = HostRelationalPersistence::default();

        let _ = host_persistence.delete(&new_host.id).await.unwrap();

        let inserted_host_id = host_persistence.create(new_host).await.unwrap();

        let fetched_host = host_persistence
            .get_by_id(&inserted_host_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_host.cpu_capacity, 4000);

        let deleted_hosts = host_persistence.delete(&inserted_host_id).await.unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
