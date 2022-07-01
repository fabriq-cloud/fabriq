pub mod api;
mod event_stream;
pub mod git;
mod protobufs;

pub use event_stream::EventStream;
use prost::Message;
pub use protobufs::*;

pub fn get_current_or_previous_model<ModelMessage: Default + Message>(
    event: &Event,
) -> anyhow::Result<ModelMessage> {
    let message: ModelMessage =
        if let Some(serialized_current_model) = &event.serialized_current_model {
            ModelMessage::decode(&**serialized_current_model)?
        } else if let Some(serialized_previous_model) = &event.serialized_previous_model {
            ModelMessage::decode(&**serialized_previous_model)?
        } else {
            return Err(anyhow::anyhow!(
                "Event received without previous or current model {:?}",
                event
            ));
        };

    Ok(message)
}
