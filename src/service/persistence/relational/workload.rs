use diesel::prelude::*;

use crate::persistence::{Persistence, WorkloadPersistence};
use crate::schema::workloads::table;
use crate::{models::Workload, schema::workloads, schema::workloads::dsl::*};

#[derive(Default, Debug)]
pub struct WorkloadRelationalPersistence {}

impl Persistence<Workload> for WorkloadRelationalPersistence {
    #[tracing::instrument(name = "relational::workload::create")]
    fn create(&self, workload: &Workload) -> anyhow::Result<String> {
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

    #[tracing::instrument(name = "relational::workload::create_many")]
    fn create_many(&self, models: &[Workload]) -> anyhow::Result<Vec<String>> {
        let connection = crate::db::get_connection()?;

        let results = diesel::insert_into(table)
            .values(models)
            .returning(workloads::id)
            .get_results(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::workload::delete")]
    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(workloads.filter(id.eq(model_id))).execute(&connection)?)
    }

    #[tracing::instrument(name = "relational::workload::delete_many")]
    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, model_id) in model_ids.iter().enumerate() {
            self.delete(model_id)?;
        }

        Ok(model_ids.len())
    }

    #[tracing::instrument(name = "relational::workload::list")]
    fn list(&self) -> anyhow::Result<Vec<Workload>> {
        let connection = crate::db::get_connection()?;

        let results = workloads.load::<Workload>(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::workload::get_by_id")]
    fn get_by_id(&self, workload_id: &str) -> anyhow::Result<Option<Workload>> {
        let connection = crate::db::get_connection()?;

        let results = workloads
            .filter(id.eq(workload_id))
            .load::<Workload>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }
}

impl WorkloadPersistence for WorkloadRelationalPersistence {
    #[tracing::instrument(name = "relational::workload::get_by_template_id")]
    fn get_by_template_id(&self, query_template_id: &str) -> anyhow::Result<Vec<Workload>> {
        let connection = crate::db::get_connection()?;

        let results = workloads
            .filter(template_id.eq(query_template_id))
            .load::<Workload>(&connection)?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::get_workload_fixture;

    use super::*;
    use crate::models::Workload;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let workload_persistence = WorkloadRelationalPersistence::default();
        let workload: Workload = get_workload_fixture(Some("relational-workload-create")).into();

        workload_persistence.delete(&workload.id).unwrap();

        let inserted_workload_id = workload_persistence.create(&workload).unwrap();

        let fetched_workload = workload_persistence
            .get_by_id(&inserted_workload_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_workload.id, workload.id);

        let deleted_workloads = workload_persistence.delete(&inserted_workload_id).unwrap();
        assert_eq!(deleted_workloads, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let workload_persistence = WorkloadRelationalPersistence::default();
        let workload: Workload =
            get_workload_fixture(Some("relational-workload-create-many")).into();

        workload_persistence.delete(&workload.id).unwrap();

        let inserted_workload_ids = workload_persistence
            .create_many(&[workload.clone()])
            .unwrap();
        assert_eq!(inserted_workload_ids.len(), 1);
        assert_eq!(inserted_workload_ids[0], workload.id);

        let deleted_workloads = workload_persistence.delete_many(&[&workload.id]).unwrap();
        assert_eq!(deleted_workloads, 1);
    }
}
