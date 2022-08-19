use diesel::pg::upsert::excluded;
use diesel::prelude::*;

use crate::persistence::Persistence;
use crate::schema::templates::table;
use crate::{models::Template, schema::templates::dsl::*};

#[derive(Default, Debug)]
pub struct TemplateRelationalPersistence {}

impl Persistence<Template> for TemplateRelationalPersistence {
    #[tracing::instrument(name = "relational::template::create_many")]
    fn create(&self, template: &Template) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        let created_count = diesel::insert_into(table)
            .values(template)
            .on_conflict(id)
            .do_update()
            .set((
                repository.eq(template.repository.clone()),
                git_ref.eq(template.git_ref.clone()),
                path.eq(template.path.clone()),
            ))
            .execute(&connection)?;

        Ok(created_count)
    }

    #[tracing::instrument(name = "relational::template::create_many")]
    fn create_many(&self, models: &[Template]) -> anyhow::Result<usize> {
        let connection = crate::db::get_connection()?;

        let results = diesel::insert_into(table)
            .values(models)
            .on_conflict(id)
            .do_update()
            .set((
                repository.eq(excluded(repository)),
                git_ref.eq(excluded(git_ref)),
                path.eq(excluded(path)),
            ))
            .execute(&connection)?;

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
        template_persistence.delete(&template.id).unwrap();

        let created_count = template_persistence.create(&template).unwrap();

        assert_eq!(created_count, 1);

        let fetched_template = template_persistence
            .get_by_id(&template.id)
            .unwrap()
            .unwrap();

        assert_eq!(fetched_template.id, template.id);
        assert_eq!(fetched_template.repository, template.repository);

        let deleted_templates = template_persistence.delete(&template.id).unwrap();

        assert_eq!(deleted_templates, 1);
    }

    #[test]
    fn test_create_get_delete_many() {
        dotenv::from_filename(".env.test").ok();
        crate::persistence::relational::ensure_fixtures();

        let template_persistence = TemplateRelationalPersistence::default();
        let template: Template =
            get_template_fixture(Some("relational-template-create-many")).into();

        let created_count = template_persistence
            .create_many(&[template.clone()])
            .unwrap();

        assert_eq!(created_count, 1);

        let deleted_templates = template_persistence.delete_many(&[&template.id]).unwrap();

        assert_eq!(deleted_templates, 1);
    }
}
