use super::Event;

pub trait EventStream: Send + Sync {
    fn receive(&self) -> Box<dyn Iterator<Item = Option<Event>> + '_>;
    fn send(&self, event: &Event) -> anyhow::Result<()>;
    fn send_many(&self, events: &[Event]) -> anyhow::Result<()>;
}
