use akira_core::Persistence;
use diesel::prelude::*;

use crate::schema::deployments::table;
use crate::{models::Deployment, schema::deployments, schema::deployments::dsl::*};

#[derive(Default)]
pub struct DeploymentRelationalPersistence {}

impl Persistence<Deployment> for DeploymentRelationalPersistence {
    fn create(&self, deployment: Deployment) -> anyhow::Result<String> {
        let connection = crate::db::get_connection()?;

        let results: Vec<String> = diesel::insert_into(table)
            .values(deployment)
            .returning(deployments::id)
            .get_results(&connection)?;

        match results.first() {
            Some(host_id) => Ok(host_id.clone()),
            None => Err(anyhow::anyhow!("Couldn't find created host id returned")),
        }
    }

    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(deployments.filter(id.eq(model_id))).execute(&connection)?)
    }

    fn list(&self) -> anyhow::Result<Vec<Deployment>> {
        let connection = crate::db::get_connection()?;

        let results = deployments.load::<Deployment>(&connection).unwrap();

        Ok(results)
    }

    fn get_by_id(&self, deployment_id: &str) -> anyhow::Result<Option<Deployment>> {
        let connection = crate::db::get_connection()?;

        let results = deployments
            .filter(id.eq(deployment_id))
            .load::<Deployment>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;
    use crate::models::Deployment;

    #[test]
    fn test_create_get_delete() {
        dotenv().ok();
        crate::persistence::relational::ensure_fixtures();

        let new_deployment = Deployment {
            id: "deployment-under-test".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            target_id: "target-fixture".to_owned(),
            hosts: 2,
        };

        let deployment_persistence = DeploymentRelationalPersistence::default();

        // delete deployment if it exists
        let _ = deployment_persistence.delete(&new_deployment.id).unwrap();

        let inserted_deployment_id = deployment_persistence
            .create(new_deployment.clone())
            .unwrap();

        let fetched_deployment = deployment_persistence
            .get_by_id(&inserted_deployment_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_deployment.id, new_deployment.id);

        let deleted_deployments = deployment_persistence
            .delete(&inserted_deployment_id)
            .unwrap();
        assert_eq!(deleted_deployments, 1);
    }
}
