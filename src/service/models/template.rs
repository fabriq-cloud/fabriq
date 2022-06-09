use akira_core::{PersistableModel, TemplateMessage};

use crate::schema::templates;

#[derive(Clone, Debug, Insertable, Eq, PartialEq, Queryable, QueryableByName)]
#[table_name = "templates"]
pub struct Template {
    pub id: String, // external-service
    pub repository: String,
    pub branch: String,
    pub path: String,
}

impl PersistableModel<Template, Template> for Template {
    fn new(new_template: Template) -> Self {
        new_template
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Template> for TemplateMessage {
    fn from(template: Template) -> Self {
        Self {
            id: template.id,
            repository: template.repository,
            branch: template.branch,
            path: template.path,
        }
    }
}
