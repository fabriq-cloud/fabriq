mod assignment;
mod config;
mod deployment;
mod host;
mod target;
mod template;
mod workload;

pub use assignment::AssignmentRelationalPersistence;
pub use config::ConfigRelationalPersistence;
pub use deployment::DeploymentRelationalPersistence;
pub use host::HostRelationalPersistence;
pub use target::TargetRelationalPersistence;
pub use template::TemplateRelationalPersistence;
pub use workload::WorkloadRelationalPersistence;

#[cfg(test)]
pub mod tests {
    use lazy_static::lazy_static;
    use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
    use std::sync::{Arc, Mutex};

    use crate::{
        models::{Deployment, Host, Target, Template, Workload},
        persistence::{
            relational::{
                DeploymentRelationalPersistence, HostRelationalPersistence,
                TargetRelationalPersistence, TemplateRelationalPersistence,
                WorkloadRelationalPersistence,
            },
            Persistence,
        },
    };

    lazy_static! {
        pub static ref FIXTURES_CREATED: Mutex<bool> = Mutex::new(false);
    }

    #[cfg(test)]
    pub async fn ensure_fixtures() -> Arc<Pool<Postgres>> {
        use fabriq_core::test::{
            get_deployment_fixture, get_host_fixture, get_target_fixture, get_template_fixture,
            get_workload_fixture,
        };

        let database_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let db = Arc::new(
            PgPoolOptions::new()
                .max_connections(1)
                .connect(&database_url)
                .await
                .expect("failed to connect to DATABASE_URL"),
        );

        let result = sqlx::migrate!().run(&*db).await;

        if let Err(err) = result {
            tracing::error!("err: {:?}", err);
        }

        {
            // tests run multithreaded, so we need to ensure that we block all but the first
            let mut fixtures_created = FIXTURES_CREATED.lock().unwrap();

            if *fixtures_created {
                // fixtures already created
                return db;
            } else {
                *fixtures_created = true;
            }
        }

        let host_persistence = HostRelationalPersistence {
            db: Arc::clone(&db),
        };
        let host_fixture: Host = get_host_fixture(None).into();
        let host_fixture = host_persistence.get_by_id(&host_fixture.id).await.unwrap();

        if host_fixture.is_none() {
            let host_fixture: Host = get_host_fixture(None).into();
            host_persistence.upsert(&host_fixture).await.unwrap();
        }

        let target_persistence = TargetRelationalPersistence {
            db: Arc::clone(&db),
        };
        let target_fixture: Target = get_target_fixture(None).into();
        let target_fixture = target_persistence
            .get_by_id(&target_fixture.id)
            .await
            .unwrap();

        if target_fixture.is_none() {
            let target_fixture: Target = get_target_fixture(None).into();
            target_persistence.upsert(&target_fixture).await.unwrap();
        }

        let template_persistence = TemplateRelationalPersistence {
            db: Arc::clone(&db),
        };
        let template_fixture: Template = get_template_fixture(None).into();
        let template_fixture = template_persistence
            .get_by_id(&template_fixture.id)
            .await
            .unwrap();

        if template_fixture.is_none() {
            let template_fixture: Template = get_template_fixture(None).into();
            template_persistence
                .upsert(&template_fixture)
                .await
                .unwrap();
        }

        let workload_persistence = WorkloadRelationalPersistence {
            db: Arc::clone(&db),
        };
        let workload_fixture: Workload = get_workload_fixture(None).into();
        let workload = workload_persistence
            .get_by_id(&workload_fixture.id)
            .await
            .unwrap();

        if workload.is_none() {
            let workload_fixture: Workload = get_workload_fixture(None).into();

            workload_persistence
                .upsert(&workload_fixture)
                .await
                .unwrap();
        }

        let deployment_persistence = DeploymentRelationalPersistence {
            db: Arc::clone(&db),
        };
        let deployment_fixture: Deployment = get_deployment_fixture(None).into();
        let deployment = deployment_persistence
            .get_by_id(&deployment_fixture.id)
            .await
            .unwrap();

        if deployment.is_none() {
            let deployment_fixture: Deployment = get_deployment_fixture(None).into();

            deployment_persistence
                .upsert(&deployment_fixture)
                .await
                .unwrap();
        }

        db
    }
}
