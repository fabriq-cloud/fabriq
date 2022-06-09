use async_trait::async_trait;

use super::Event;

#[async_trait]
pub trait EventStream: Send + Sync {
    async fn fill_operation_id(&self, current_operation_id: Option<i32>) -> anyhow::Result<i32>;
    async fn receive(&self) -> anyhow::Result<Option<Event>>;
    async fn send(&self, event: &Event) -> anyhow::Result<()>;
    async fn len(&self) -> anyhow::Result<usize>;
}
