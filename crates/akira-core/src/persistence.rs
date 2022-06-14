pub trait Persistence<Model>: Send + Sync {
    fn create(&self, model: &Model) -> anyhow::Result<String>;
    fn create_many(&self, models: &[Model]) -> anyhow::Result<Vec<String>>;
    fn delete(&self, model_id: &str) -> anyhow::Result<usize>;
    fn delete_many(&self, model_ids: &[&str]) -> anyhow::Result<usize>;
    fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Model>>;
    fn list(&self) -> anyhow::Result<Vec<Model>>;
}

pub trait PersistableModel<Model> {
    #[allow(clippy::all)]
    fn new(new_model: &Model) -> Model;
    fn get_id(&self) -> String;
}
