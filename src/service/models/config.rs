use akira_core::ConfigMessage;

use crate::{persistence::PersistableModel, schema::configs};

#[derive(
    Associations,
    Clone,
    Debug,
    Default,
    Eq,
    Identifiable,
    Insertable,
    PartialEq,
    Queryable,
    QueryableByName,
)]
#[table_name = "configs"]
pub struct Config {
    pub id: String,

    pub owning_model: String,

    pub key: String,
    pub value: String,
}

impl PersistableModel<Config> for Config {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl From<Config> for ConfigMessage {
    fn from(config: Config) -> Self {
        Self {
            id: config.id,
            owning_model: config.owning_model,

            key: config.key,
            value: config.value,
        }
    }
}

impl From<ConfigMessage> for Config {
    fn from(config: ConfigMessage) -> Self {
        Self {
            id: config.id,
            owning_model: config.owning_model,

            key: config.key,
            value: config.value,
        }
    }
}

impl Config {
    pub fn make_owning_model(model_type: &str, model_id: &str) -> String {
        format!("{}:{}", model_type, model_id)
    }

    pub fn split_owning_model(&self) -> (String, String) {
        let mut split = self.owning_model.split(':');
        (
            split.next().unwrap().to_owned(),
            split.next().unwrap().to_owned(),
        )
    }
}
