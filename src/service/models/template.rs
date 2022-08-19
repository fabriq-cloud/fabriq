use akira_core::TemplateMessage;

use crate::{persistence::PersistableModel, schema::templates};

#[derive(Clone, Debug, Default, Insertable, Eq, PartialEq, Queryable, QueryableByName)]
#[table_name = "templates"]
pub struct Template {
    pub id: String, // external-service
    pub repository: String,
    pub git_ref: String,
    pub path: String,
}

impl PersistableModel<Template> for Template {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Template> for TemplateMessage {
    fn from(template: Template) -> Self {
        Self {
            id: template.id,
            repository: template.repository,
            git_ref: template.git_ref,
            path: template.path,
        }
    }
}

impl From<TemplateMessage> for Template {
    fn from(template: TemplateMessage) -> Self {
        Self {
            id: template.id,
            repository: template.repository,
            git_ref: template.git_ref,
            path: template.path,
        }
    }
}
