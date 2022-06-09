use akira_core::Persistence;
use async_trait::async_trait;
use diesel::prelude::*;

use crate::schema::targets::table;
use crate::{models::Target, schema::targets, schema::targets::dsl::*};

#[derive(Default)]
pub struct TargetRelationalPersistence {}

#[async_trait]
impl Persistence<Target, Target> for TargetRelationalPersistence {
    async fn create(&self, target: Target) -> anyhow::Result<String> {
        let connection = crate::db::get_connection()?;

        let results: Vec<String> = diesel::insert_into(table)
            .values(target)
            .returning(targets::id)
            .get_results(&connection)?;

        match results.first() {
            Some(host_id) => Ok(host_id.clone()),
            None => Err(anyhow::anyhow!("Couldn't find created host id returned")),
        }
    }

    async fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(targets.filter(id.eq(model_id))).execute(&connection)?)
    }

    async fn list(&self) -> anyhow::Result<Vec<Target>> {
        let connection = crate::db::get_connection()?;

        let results = targets.load::<Target>(&connection).unwrap();

        Ok(results)
    }

    async fn get_by_id(&self, target_id: &str) -> anyhow::Result<Option<Target>> {
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
    use dotenv::dotenv;

    use super::*;
    use crate::models::Target;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        let new_target = Target {
            id: "target-under-test".to_owned(),
            labels: vec!["cloud:azure".to_string()],
        };

        let target_persistence = TargetRelationalPersistence::default();

        // delete target if it exists
        let _ = target_persistence.delete(&new_target.id).await.unwrap();

        let inserted_target_id = target_persistence.create(new_target.clone()).await.unwrap();

        let fetched_target = target_persistence
            .get_by_id(&inserted_target_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_target.id, new_target.id);

        let deleted_targets = target_persistence
            .delete(&inserted_target_id)
            .await
            .unwrap();
        assert_eq!(deleted_targets, 1);
    }
}
