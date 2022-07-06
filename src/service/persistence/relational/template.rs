use diesel::prelude::*;

use crate::persistence::Persistence;
use crate::schema::templates::table;
use crate::{models::Template, schema::templates, schema::templates::dsl::*};

#[derive(Default, Debug)]
pub struct TemplateRelationalPersistence {}

impl Persistence<Template> for TemplateRelationalPersistence {
    #[tracing::instrument(name = "relational::template::create_many")]
    fn create(&self, template: &Template) -> anyhow::Result<String> {
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

    #[tracing::instrument(name = "relational::template::create_many")]
    fn create_many(&self, models: &[Template]) -> anyhow::Result<Vec<String>> {
        let connection = crate::db::get_connection()?;

        let results = diesel::insert_into(table)
            .values(models)
            .returning(templates::id)
            .get_results(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::template::delete")]
    fn delete(&self, model_id: &str) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        Ok(diesel::delete(templates.filter(id.eq(model_id))).execute(&connection)?)
    }

    #[tracing::instrument(name = "relational::template::delete_many")]
    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize> {
        for (_, model_id) in model_ids.iter().enumerate() {
            self.delete(model_id)?;
        }

        Ok(model_ids.len())
    }

    #[tracing::instrument(name = "relational::template::list")]
    fn list(&self) -> anyhow::Result<Vec<Template>> {
        let connection = crate::db::get_connection()?;

        let results = templates.load::<Template>(&connection)?;

        Ok(results)
    }

    #[tracing::instrument(name = "relational::template::get_by_id")]
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
    use super::*;
    use crate::models::Template;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let new_template = Template {
            id: "template-under-test".to_owned(),
            repository: "http://github.com/timfpark/deployment-templates".to_owned(),
            branch: "main".to_owned(),
            path: "external-service".to_owned(),
        };

        let template_persistence = TemplateRelationalPersistence::default();

        // delete template if it exists
        let _ = template_persistence.delete(&new_template.id).unwrap();

        let inserted_template_id = template_persistence.create(&new_template).unwrap();

        let fetched_template = template_persistence
            .get_by_id(&inserted_template_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_template.id, new_template.id);
        assert_eq!(fetched_template.repository, new_template.repository);

        let deleted_templates = template_persistence.delete(&inserted_template_id).unwrap();
        assert_eq!(deleted_templates, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let new_template = Template {
            id: "template-under-many-test".to_owned(),
            repository: "http://github.com/timfpark/deployment-templates".to_owned(),
            branch: "main".to_owned(),
            path: "external-service".to_owned(),
        };

        let template_persistence = TemplateRelationalPersistence::default();

        let inserted_template_ids = template_persistence
            .create_many(&[new_template.clone()])
            .unwrap();
        assert_eq!(inserted_template_ids.len(), 1);
        assert_eq!(inserted_template_ids[0], new_template.id);

        let deleted_templates = template_persistence
            .delete_many(&[&new_template.id])
            .unwrap();
        assert_eq!(deleted_templates, 1);
    }
}
