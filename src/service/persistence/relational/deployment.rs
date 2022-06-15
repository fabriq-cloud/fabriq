use diesel::prelude::*;

use crate::persistence::{DeploymentPersistence, Persistence};
use crate::schema::deployments::table;
use crate::{models::Deployment, schema::deployments, schema::deployments::dsl::*};

#[derive(Default)]
pub struct DeploymentRelationalPersistence {}

impl Persistence<Deployment> for DeploymentRelationalPersistence {
    fn create(&self, deployment: &Deployment) -> anyhow::Result<String> {
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

    fn create_many(&self, models: &[Deployment]) -> anyhow::Result<Vec<String>> {
        let connection = crate::db::get_connection()?;

        let results = diesel::insert_into(table)
            .values(models)
            .returning(deployments::id)
            .get_results(&connection)?;

        Ok(results)
    }

    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(deployments.filter(id.eq(model_id))).execute(&connection)?)
    }

    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, model_id) in model_ids.iter().enumerate() {
            self.delete(model_id)?;
        }

        Ok(model_ids.len())
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

    #[cfg(test)]
    fn ensure_fixtures(&self) -> anyhow::Result<()> {
        let deployment_fixture = Deployment {
            id: "deployment-fixture".to_string(),
            workload_id: "workload-fixture".to_string(),
            target_id: "target-fixture".to_string(),
            hosts: 2,
        };

        self.create(&deployment_fixture)?;

        Ok(())
    }
}

impl DeploymentPersistence for DeploymentRelationalPersistence {
    fn get_by_target_id(&self, query_target_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let connection = crate::db::get_connection()?;

        let results = deployments
            .filter(target_id.eq(query_target_id))
            .load::<Deployment>(&connection)?;

        Ok(results)
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

        let new_deployment = Deployment {
            id: "deployment-under-test".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            target_id: "target-fixture".to_owned(),
            hosts: 2,
        };

        let deployment_persistence = DeploymentRelationalPersistence::default();

        // delete deployment if it exists
        let _ = deployment_persistence.delete(&new_deployment.id).unwrap();

        let inserted_deployment_id = deployment_persistence.create(&new_deployment).unwrap();

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

    #[test]
    fn test_create_get_delete_many() {
        dotenv().ok();

        let new_deployment = Deployment {
            id: "deployment-under-many-test".to_owned(),
            workload_id: "workload-fixture".to_owned(),
            target_id: "target-fixture".to_owned(),
            hosts: 2,
        };

        let deployment_persistence = DeploymentRelationalPersistence::default();

        let inserted_deployment_ids = deployment_persistence
            .create_many(&[new_deployment.clone()])
            .unwrap();
        assert_eq!(inserted_deployment_ids.len(), 1);
        assert_eq!(inserted_deployment_ids[0], new_deployment.id);

        let deleted_deployments = deployment_persistence
            .delete_many(&[&new_deployment.id])
            .unwrap();
        assert_eq!(deleted_deployments, 1);
    }

    #[test]
    fn test_get_by_target_id() {
        dotenv().ok();

        let deployment_persistence = DeploymentRelationalPersistence::default();

        let deployments_for_target = deployment_persistence
            .get_by_target_id("target-fixture")
            .unwrap();

        assert_eq!(deployments_for_target.len(), 1);
    }
}
