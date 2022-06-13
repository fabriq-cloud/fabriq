use std::{sync::Arc, time::Duration};

use tokio::time::sleep;

use super::{EventStream, Processor};

pub struct Dispatcher {
    event_stream: Arc<Box<dyn EventStream>>,
    processors: Vec<Box<dyn Processor>>,
}

impl Dispatcher {
    pub fn new(
        event_stream: Arc<Box<dyn EventStream>>,
        processors: Vec<Box<dyn Processor>>,
    ) -> Self {
        Dispatcher {
            event_stream,
            processors,
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        loop {
            let event = self.event_stream.receive()?;

            if let Some(event) = event {
                tracing::info!("processing event: {:?}", event);
                for processor in &self.processors {
                    processor.process(&event).await?;
                }
            } else {
                tracing::info!("empty event queue");
                sleep(Duration::from_secs(1)).await;
            }

            // always yield time to other tasks, in particular the api, between events
            sleep(Duration::from_millis(1)).await;
        }
    }
}
