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
    use akira_core::test::get_template_fixture;

    use super::*;
    use crate::models::Template;

    #[test]
    fn test_create_get_delete() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let template_persistence = TemplateRelationalPersistence::default();
        let template: Template = get_template_fixture(Some("relational-template-create")).into();

        // delete template if it exists
        let _ = template_persistence.delete(&template.id).unwrap();

        let inserted_template_id = template_persistence.create(&template).unwrap();

        let fetched_template = template_persistence
            .get_by_id(&inserted_template_id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_template.id, template.id);
        assert_eq!(fetched_template.repository, template.repository);

        let deleted_templates = template_persistence.delete(&inserted_template_id).unwrap();
        assert_eq!(deleted_templates, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let template_persistence = TemplateRelationalPersistence::default();
        let template: Template =
            get_template_fixture(Some("relational-template-create-many")).into();

        let inserted_template_ids = template_persistence
            .create_many(&[template.clone()])
            .unwrap();
        assert_eq!(inserted_template_ids.len(), 1);
        assert_eq!(inserted_template_ids[0], template.id);

        let deleted_templates = template_persistence.delete_many(&[&template.id]).unwrap();
        assert_eq!(deleted_templates, 1);
    }
}
