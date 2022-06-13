use akira_core::Persistence;
use async_trait::async_trait;
use diesel::prelude::*;

use crate::schema::workspaces::table;
use crate::{models::Workspace, schema::workspaces, schema::workspaces::dsl::*};

#[derive(Default)]
pub struct WorkspaceRelationalPersistence {}

#[async_trait]
impl Persistence<Workspace> for WorkspaceRelationalPersistence {
    fn create(&self, workspace: Workspace) -> anyhow::Result<String> {
        let connection = crate::db::get_connection()?;

        let results: Vec<String> = diesel::insert_into(table)
            .values(workspace)
            .returning(workspaces::id)
            .get_results(&connection)?;

        match results.first() {
            Some(host_id) => Ok(host_id.clone()),
            None => Err(anyhow::anyhow!("Couldn't find created host id returned")),
        }
    }

    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(workspaces.filter(id.eq(model_id))).execute(&connection)?)
    }

    fn list(&self) -> anyhow::Result<Vec<Workspace>> {
        let connection = crate::db::get_connection()?;

        let results = workspaces.load::<Workspace>(&connection).unwrap();

        Ok(results)
    }

    fn get_by_id(&self, workspace_id: &str) -> anyhow::Result<Option<Workspace>> {
        let connection = crate::db::get_connection()?;

        let results = workspaces
            .filter(id.eq(workspace_id))
            .load::<Workspace>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;
    use crate::models::Workspace;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        let new_workspace = Workspace {
            id: "workspace-under-test".to_owned(),
        };

        let workspace_persistence = WorkspaceRelationalPersistence::default();

        // delete workspace if it exists
        let _ = workspace_persistence.delete(&new_workspace.id).unwrap();

        let inserted_workspace_id = workspace_persistence.create(new_workspace.clone()).unwrap();

        let fetched_workspace = workspace_persistence
            .get_by_id(&inserted_workspace_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_workspace.id, new_workspace.id);

        let deleted_workspaces = workspace_persistence
            .delete(&inserted_workspace_id)
            .unwrap();
        assert_eq!(deleted_workspaces, 1);
    }
}
