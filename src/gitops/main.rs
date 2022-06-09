use std::sync::Arc;

mod context;
mod processor;

use akira_core::{Dispatcher, EventStream};
use akira_memory_stream::MemoryEventStream;
use processor::GitOpsProcessor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let event_stream: Arc<Box<dyn EventStream>> = Arc::new(Box::new(MemoryEventStream::new()?));

    let gitops_processor = Box::new(GitOpsProcessor::new().await?);
    let dispatcher = Dispatcher::new(Arc::clone(&event_stream), vec![gitops_processor]);

    dispatcher.start().await
}
