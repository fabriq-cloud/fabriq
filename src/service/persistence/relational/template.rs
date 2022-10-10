use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::{models::Template, persistence::Persistence};

#[derive(Debug)]
pub struct TemplateRelationalPersistence {
    pub db: Arc<PgPool>,
}

#[async_trait]
impl Persistence<Template> for TemplateRelationalPersistence {
    #[tracing::instrument(name = "relational::template::create_many")]
    async fn upsert(&self, template: &Template) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO templates
               (id, repository, git_ref, path)
            VALUES
               ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
               repository = $2,
               git_ref = $3,
               path = $4
            "#,
            template.id,
            template.repository,
            template.git_ref,
            template.path
        )
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::template::delete")]
    async fn delete(&self, id: &str) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM templates WHERE id = $1
            "#,
            id
        )
        .bind(id)
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "relational::template::list")]
    async fn list(&self) -> anyhow::Result<Vec<Template>> {
        let rows = sqlx::query_as!(
            Template,
            r#"
                SELECT * FROM templates
            "#,
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows
            .into_iter()
            .map(Template::from)
            .collect::<Vec<Template>>();

        Ok(models)
    }

    #[tracing::instrument(name = "relational::template::get_by_id")]
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Template>> {
        let supply = sqlx::query_as!(Template, "SELECT * FROM templates WHERE id = $1", id)
            .fetch_optional(&*self.db)
            .await?;

        Ok(supply)
    }
}

#[cfg(test)]
mod tests {
    use fabriq_core::test::get_template_fixture;

    use super::*;
    use crate::models::Template;

    use crate::persistence::relational::tests::ensure_fixtures;

    #[tokio::test]
    async fn test_create_get_delete() {
        dotenvy::from_filename(".env.test").ok();
        let db = ensure_fixtures().await;

        let template_persistence = TemplateRelationalPersistence { db };
        let template: Template = get_template_fixture(Some("relational-template-create")).into();

        // delete template if it exists
        template_persistence.delete(&template.id).await.unwrap();

        let created_count = template_persistence.upsert(&template).await.unwrap();

        assert_eq!(created_count, 1);

        let fetched_template = template_persistence
            .get_by_id(&template.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_template.id, template.id);
        assert_eq!(fetched_template.repository, template.repository);

        let deleted_templates = template_persistence.delete(&template.id).await.unwrap();

        assert_eq!(deleted_templates, 1);
    }
}
