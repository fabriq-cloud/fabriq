use diesel::sql_types::SmallInt;

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

    #[sql_type = "SmallInt"]
    pub model_type: i16,
    pub model_id: String,

    pub key: String,
    pub value: String,
}

impl PersistableModel<Config> for Config {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}
