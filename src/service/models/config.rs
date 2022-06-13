use akira_core::PersistableModel;
use diesel::sql_types::SmallInt;

use crate::schema::configs;

#[derive(
    Associations, Clone, Debug, Eq, Identifiable, Insertable, PartialEq, Queryable, QueryableByName,
)]
#[table_name = "configs"]
pub struct Config {
    pub id: String,

    #[sql_type = "SmallInt"]
    pub model_type: i16,
    pub model_id: String,

    pub key: String,
    pub value: String,
}

impl PersistableModel<Config> for Config {
    fn new(new_config: Config) -> Self {
        new_config
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }
}
