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

    pub value_type: i32,
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

            value_type: config.value_type,
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

            value_type: config.value_type,
        }
    }
}

impl Config {
    pub fn split_owning_model(&self) -> (String, String) {
        let mut split = self
            .owning_model
            .split(ConfigMessage::OWNING_MODEL_SEPARATOR);
        (
            split.next().unwrap().to_owned(),
            split.next().unwrap().to_owned(),
        )
    }
}
