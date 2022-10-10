use async_trait::async_trait;
use std::fmt::Debug;

use super::Event;

#[async_trait]
pub trait EventStream: Debug + Send + Sync {
    async fn delete(&self, event: &Event, consumer_id: &str) -> anyhow::Result<u64>;
    async fn receive(&self, consumer_id: &str) -> anyhow::Result<Vec<Event>>;
    async fn send(&self, event: &Event) -> anyhow::Result<()>;
    async fn send_many(&self, events: &[Event]) -> anyhow::Result<()>;
}
