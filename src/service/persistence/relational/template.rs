use akira_core::Persistence;
use async_trait::async_trait;
use diesel::prelude::*;

use crate::schema::templates::table;
use crate::{models::Template, schema::templates, schema::templates::dsl::*};

#[derive(Default)]
pub struct TemplateRelationalPersistence {}

#[async_trait]
impl Persistence<Template> for TemplateRelationalPersistence {
    fn create(&self, template: Template) -> anyhow::Result<String> {
        let connection = crate::db::get_connection()?;

        let results: Vec<String> = diesel::insert_into(table)
            .values(template)
            .returning(templates::id)
            .get_results(&connection)?;

        match results.first() {
            Some(host_id) => Ok(host_id.clone()),
            None => Err(anyhow::anyhow!("Couldn't find created host id returned")),
        }
    }

    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(templates.filter(id.eq(model_id))).execute(&connection)?)
    }

    fn list(&self) -> anyhow::Result<Vec<Template>> {
        let connection = crate::db::get_connection()?;

        let results = templates.load::<Template>(&connection).unwrap();

        Ok(results)
    }

    fn get_by_id(&self, template_id: &str) -> anyhow::Result<Option<Template>> {
        let connection = crate::db::get_connection()?;

        let results = templates
            .filter(id.eq(template_id))
            .load::<Template>(&connection)?;

        let cloned_result = results.first().cloned();

        Ok(cloned_result)
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;
    use crate::models::Template;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenv().ok();

        let new_template = Template {
            id: "template-under-test".to_owned(),
            repository: "http://github.com/timfpark/deployment-templates".to_owned(),
            branch: "main".to_owned(),
            path: "external-service".to_owned(),
        };

        let template_persistence = TemplateRelationalPersistence::default();

        // delete template if it exists
        let _ = template_persistence.delete(&new_template.id).unwrap();

        let inserted_template_id = template_persistence.create(new_template.clone()).unwrap();

        let fetched_template = template_persistence
            .get_by_id(&inserted_template_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_template.id, new_template.id);
        assert_eq!(fetched_template.repository, new_template.repository);

        let deleted_templates = template_persistence.delete(&inserted_template_id).unwrap();
        assert_eq!(deleted_templates, 1);
    }
}
