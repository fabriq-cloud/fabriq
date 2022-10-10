use fabriq_core::TemplateMessage;

use crate::persistence::Persistable;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Template {
    pub id: String, // external-service
    pub repository: String,
    pub git_ref: String,
    pub path: String,
}

impl Persistable<Template> for Template {
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
