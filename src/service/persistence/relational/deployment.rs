use diesel::pg::upsert::excluded;
use diesel::prelude::*;

use crate::persistence::{DeploymentPersistence, Persistence};
use crate::schema::deployments::table;
use crate::{models::Deployment, schema::deployments::dsl::*};

#[derive(Default, Debug)]
pub struct DeploymentRelationalPersistence {}

impl Persistence<Deployment> for DeploymentRelationalPersistence {
    #[tracing::instrument(name = "relational::deployment::create")]
    fn create(&self, deployment: &Deployment) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        let changed_count = diesel::insert_into(table)
            .values(deployment)
            .on_conflict(id)
            .do_update()
            .set((
                name.eq(deployment.name.clone()),
                workload_id.eq(deployment.workload_id.clone()),
                target_id.eq(deployment.target_id.clone()),
                template_id.eq(deployment.template_id.clone()),
                host_count.eq(deployment.host_count),
            ))
            .execute(&connection)?;

        Ok(changed_count)
    }

    #[tracing::instrument(name = "relational::deployment::create_many")]
    fn create_many(&self, models: &[Deployment]) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        let changed_count = diesel::insert_into(table)
            .values(models)
            .on_conflict(id)
            .do_update()
            .set((
                name.eq(excluded(name)),
                workload_id.eq(excluded(workload_id)),
                target_id.eq(excluded(target_id)),
                template_id.eq(excluded(template_id)),
                host_count.eq(excluded(host_count)),
            ))
            .execute(&connection)?;

        Ok(changed_count)
    }

    #[tracing::instrument(name = "relational::deployment::delete")]
    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(deployments.filter(id.eq(model_id))).execute(&connection)?)
    }

    #[tracing::instrument(name = "relational::deployment::delete_many")]
    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, model_id) in model_ids.iter().enumerate() {
            self.delete(model_id)?;
        }

        Ok(model_ids.len())
    }

    #[tracing::instrument(name = "relational::deployment::list")]
    fn list(&self) -> anyhow::Result<Vec<Deployment>> {
        let connection = crate::db::get_connection()?;

        let results = deployments.load::<Deployment>(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::deployment::get_by_id")]
    fn get_by_id(&self, deployment_id: &str) -> anyhow::Result<Option<Deployment>> {
        let connection = crate::db::get_connection()?;

        let results = deployments
            .filter(id.eq(deployment_id))
            .load::<Deployment>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }
}

impl DeploymentPersistence for DeploymentRelationalPersistence {
    #[tracing::instrument(name = "relational::deployment::get_by_target_id")]
    fn get_by_target_id(&self, query_target_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let connection = crate::db::get_connection()?;

        let results = deployments
            .filter(target_id.eq(query_target_id))
            .load::<Deployment>(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::deployment::get_by_template_id")]
    fn get_by_template_id(&self, query_template_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let connection = crate::db::get_connection()?;

        let results = deployments
            .filter(template_id.eq(query_template_id))
            .load::<Deployment>(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::deployment::get_by_workload_id")]
    fn get_by_workload_id(&self, query_workload_id: &str) -> anyhow::Result<Vec<Deployment>> {
        let connection = crate::db::get_connection()?;

        let results = deployments
            .filter(workload_id.eq(query_workload_id))
            .load::<Deployment>(&connection)?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use akira_core::test::{get_deployment_fixture, get_target_fixture, get_template_fixture};

    use super::*;
    use crate::models::Deployment;

    #[test]
    fn test_deployment_create_get_delete() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let deployment_persistence = DeploymentRelationalPersistence::default();
        let new_deployment: Deployment =
            get_deployment_fixture(Some("create-deployment-fixture")).into();

        deployment_persistence.delete(&new_deployment.id).unwrap();
        deployment_persistence.create(&new_deployment).unwrap();

        let fetched_deployment = deployment_persistence
            .get_by_id(&new_deployment.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_deployment.id, new_deployment.id);

        let deleted_deployments = deployment_persistence.delete(&new_deployment.id).unwrap();
        assert_eq!(deleted_deployments, 1);
    }

    #[test]
    fn test_deployment_create_delete_many() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let deployment_persistence = DeploymentRelationalPersistence::default();
        let new_deployment: Deployment =
            get_deployment_fixture(Some("create-many-deployment-fixture")).into();

        // delete deployment if it exists
        deployment_persistence.delete(&new_deployment.id).unwrap();

        let created_count = deployment_persistence
            .create_many(&[new_deployment.clone()])
            .unwrap();
        assert_eq!(created_count, 1);

        let deleted_deployments = deployment_persistence
            .delete_many(&[&new_deployment.id])
            .unwrap();
        assert_eq!(deleted_deployments, 1);
    }

    #[test]
    fn test_get_by_target_id() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let deployment_persistence = DeploymentRelationalPersistence::default();
        let target_fixture = get_target_fixture(None);

        let deployments_for_target = deployment_persistence
            .get_by_target_id(&target_fixture.id)
            .unwrap();

        assert!(!deployments_for_target.is_empty());
    }

    #[test]
    fn test_get_by_template_id() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let deployment_persistence = DeploymentRelationalPersistence::default();
        let template_fixture = get_template_fixture(None);

        let deployments_for_template = deployment_persistence
            .get_by_template_id(&template_fixture.id)
            .unwrap();

        assert!(!deployments_for_template.is_empty());
    }
}
