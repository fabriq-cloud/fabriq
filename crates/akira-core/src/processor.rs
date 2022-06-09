use async_trait::async_trait;

use super::Event;

#[async_trait]
pub trait Processor {
    async fn process(&self, event: &Event) -> anyhow::Result<()>;
}
