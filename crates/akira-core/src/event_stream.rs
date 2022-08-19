use std::fmt::Debug;

use super::Event;

pub trait EventStream: Debug + Send + Sync {
    fn delete(&self, event: &Event, consumer_id: &str) -> anyhow::Result<usize>;
    fn receive(&self, consumer_id: &str) -> anyhow::Result<Vec<Event>>;
    fn send(&self, event: &Event) -> anyhow::Result<()>;
    fn send_many(&self, events: &[Event]) -> anyhow::Result<()>;
}
