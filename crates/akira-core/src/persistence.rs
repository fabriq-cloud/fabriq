use async_trait::async_trait;

#[async_trait]
pub trait Persistence<Model, NewModel>: Send + Sync {
    async fn create(&self, new_model: NewModel) -> anyhow::Result<String>;
    async fn delete(&self, model_id: &str) -> anyhow::Result<usize>;
    async fn list(&self) -> anyhow::Result<Vec<Model>>;
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<Model>>;
}

pub trait PersistableModel<Model, NewModel> {
    #[allow(clippy::all)]
    fn new(new_model: NewModel) -> Model;
    fn get_id(&self) -> String;
}
