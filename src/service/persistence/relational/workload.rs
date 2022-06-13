use akira_core::Persistence;
use async_trait::async_trait;
use diesel::prelude::*;

use crate::schema::workloads::table;
use crate::{models::Workload, schema::workloads, schema::workloads::dsl::*};

#[derive(Default)]
pub struct WorkloadRelationalPersistence {}

#[async_trait]
impl Persistence<Workload> for WorkloadRelationalPersistence {
    fn create(&self, workload: Workload) -> anyhow::Result<String> {
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

    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(workloads.filter(id.eq(model_id))).execute(&connection)?)
    }

    fn list(&self) -> anyhow::Result<Vec<Workload>> {
        let connection = crate::db::get_connection()?;

        let results = workloads.load::<Workload>(&connection).unwrap();

        Ok(results)
    }

    fn get_by_id(&self, workload_id: &str) -> anyhow::Result<Option<Workload>> {
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

    #[test]
    fn test_create_get_delete() {
        dotenv().ok();

        crate::persistence::relational::ensure_fixtures();

        let new_workload = Workload {
            id: "workload-under-test".to_owned(),
            workspace_id: "workspace-fixture".to_owned(),
            template_id: "template-fixture".to_owned(),
        };

        let workload_persistence = WorkloadRelationalPersistence::default();

        // delete workload if it exists
        let _ = workload_persistence.delete(&new_workload.id).unwrap();

        let inserted_workload_id = workload_persistence.create(new_workload.clone()).unwrap();

        let fetched_workload = workload_persistence
            .get_by_id(&inserted_workload_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_workload.id, new_workload.id);

        let deleted_workloads = workload_persistence.delete(&inserted_workload_id).unwrap();
        assert_eq!(deleted_workloads, 1);
    }
}
