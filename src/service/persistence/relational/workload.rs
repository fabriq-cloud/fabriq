use akira_core::Persistence;
use async_trait::async_trait;
use diesel::prelude::*;

use crate::schema::workloads::table;
use crate::{models::Workload, schema::workloads, schema::workloads::dsl::*};

#[derive(Default)]
pub struct WorkloadRelationalPersistence {}

#[async_trait]
impl Persistence<Workload, Workload> for WorkloadRelationalPersistence {
    async fn create(&self, workload: Workload) -> anyhow::Result<String> {
        let connection = crate::db::get_connection()?;

        let results: Vec<String> = diesel::insert_into(table)
            .values(workload)
            .returning(workloads::id)
            .get_results(&connection)?;

        match results.first() {
            Some(host_id) => Ok(host_id.clone()),
            None => Err(anyhow::anyhow!("Couldn't find created host id returned")),
        }
    }

    async fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(workloads.filter(id.eq(model_id))).execute(&connection)?)
    }

    async fn list(&self) -> anyhow::Result<Vec<Workload>> {
        let connection = crate::db::get_connection()?;

        let results = workloads.load::<Workload>(&connection).unwrap();

        Ok(results)
    }

    async fn get_by_id(&self, workload_id: &str) -> anyhow::Result<Option<Workload>> {
        let connection = crate::db::get_connection()?;

        let results = workloads
            .filter(id.eq(workload_id))
            .load::<Workload>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;
    use crate::models::Workload;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        crate::persistence::relational::ensure_fixtures().await;

        let new_workload = Workload {
            id: "workload-under-test".to_owned(),
            workspace_id: "workspace-fixture".to_owned(),
            template_id: "template-fixture".to_owned(),
        };

        let workload_persistence = WorkloadRelationalPersistence::default();

        // delete workload if it exists
        let _ = workload_persistence.delete(&new_workload.id).await.unwrap();

        let inserted_workload_id = workload_persistence
            .create(new_workload.clone())
            .await
            .unwrap();

        let fetched_workload = workload_persistence
            .get_by_id(&inserted_workload_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_workload.id, new_workload.id);

        let deleted_workloads = workload_persistence
            .delete(&inserted_workload_id)
            .await
            .unwrap();
        assert_eq!(deleted_workloads, 1);
    }
}
