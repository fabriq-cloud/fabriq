use std::sync::Arc;

mod context;
mod processor;

use akira_core::EventStream;
use akira_memory_stream::MemoryEventStream;
use processor::GitOpsProcessor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _event_stream: Arc<Box<dyn EventStream>> = Arc::new(Box::new(MemoryEventStream::new()?));

    let _gitops_processor = Box::new(GitOpsProcessor::new().await?);

    Ok(())
}
