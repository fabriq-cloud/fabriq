use diesel::pg::upsert::excluded;
use diesel::prelude::*;

use crate::persistence::Persistence;
use crate::schema::targets::table;
use crate::{models::Target, schema::targets::dsl::*};

#[derive(Default, Debug)]
pub struct TargetRelationalPersistence {}

impl Persistence<Target> for TargetRelationalPersistence {
    #[tracing::instrument(name = "relational::target::create")]
    fn create(&self, target: &Target) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        let created_count = diesel::insert_into(table)
            .values(target)
            .on_conflict(id)
            .do_update()
            .set(labels.eq(target.labels.clone()))
            .execute(&connection)?;

        Ok(created_count)
    }

    #[tracing::instrument(name = "relational::target::create_many")]
    fn create_many(&self, models: &[Target]) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        let created_count = diesel::insert_into(table)
            .values(models)
            .on_conflict(id)
            .do_update()
            .set(labels.eq(excluded(labels)))
            .execute(&connection)?;

        Ok(created_count)
    }

    #[tracing::instrument(name = "relational::target::delete")]
    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(targets.filter(id.eq(model_id))).execute(&connection)?)
    }

    #[tracing::instrument(name = "relational::target::delete_many")]
    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, model_id) in model_ids.iter().enumerate() {
            self.delete(model_id)?;
        }

        Ok(model_ids.len())
    }

    #[tracing::instrument(name = "relational::target::list")]
    fn list(&self) -> anyhow::Result<Vec<Target>> {
        let connection = crate::db::get_connection()?;

        let results = targets.load::<Target>(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::target::get_by_id")]
    fn get_by_id(&self, target_id: &str) -> anyhow::Result<Option<Target>> {
        let connection = crate::db::get_connection()?;

        let results = targets
            .filter(id.eq(target_id))
            .load::<Target>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::get_target_fixture;

    use super::*;
    use crate::models::Target;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let target_persistence = TargetRelationalPersistence::default();
        let target: Target = get_target_fixture(Some("target-create")).into();

        // delete target if it exists
        target_persistence.delete(&target.id).unwrap();

        let created_count = target_persistence.create(&target).unwrap();

        assert_eq!(created_count, 1);

        let fetched_target = target_persistence.get_by_id(&target.id).unwrap().unwrap();
        assert_eq!(fetched_target.id, target.id);

        let deleted_targets = target_persistence.delete(&target.id).unwrap();
        assert_eq!(deleted_targets, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let new_target = Target {
            id: "target-under-many-test".to_owned(),
            labels: vec!["cloud:azure".to_string()],
        };

        let target_persistence = TargetRelationalPersistence::default();

        let created_count = target_persistence
            .create_many(&[new_target.clone()])
            .unwrap();
        assert_eq!(created_count, 1);

        let deleted_targets = target_persistence.delete_many(&[&new_target.id]).unwrap();
        assert_eq!(deleted_targets, 1);
    }
}
