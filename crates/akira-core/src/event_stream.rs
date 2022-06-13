use super::Event;

pub trait EventStream: Send + Sync {
    fn receive(&self) -> anyhow::Result<Option<Event>>;
    fn send(&self, event: &Event) -> anyhow::Result<()>;
    fn len(&self) -> anyhow::Result<usize>;
    fn is_empty(&self) -> anyhow::Result<bool>;
}
