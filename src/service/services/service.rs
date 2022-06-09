use std::sync::Arc;

use serde::Serialize;

use crate::eventing::{Event, EventStream, EventType};
use crate::persistence::{PersistableModel, Persistence};

pub struct Service<Model, NewModel>
where
    Model: Serialize + PersistableModel<Model, NewModel>,
{
    persistence: Box<dyn Persistence<Model, NewModel>>,
    stream: Arc<Box<dyn EventStream + 'static>>,
}

impl<Model, NewModel> Service<Model, NewModel>
where
    Model: Serialize + PersistableModel<Model, NewModel>,
{
    pub fn new(
        persistence: Box<dyn Persistence<Model, NewModel>>,
        stream: Arc<Box<dyn EventStream>>,
    ) -> Self {
        Service {
            persistence,
            stream,
        }
    }

    pub fn create(&self, model: &NewModel) -> anyhow::Result<i32> {
        let model_id = self.persistence.create(model)?;

        let model = self.get_by_id(model_id)?;
        let model = match model {
            Some(model) => model,
            None => return Err(anyhow::anyhow!("Couldn't find created model id returned")),
        };

        let create_model_event = Event::new(model.get_type(), model, EventType::Created)?;
        self.stream.send(&create_model_event)?;

        Ok(model_id)
    }

    pub fn get_by_id(&self, model_id: i32) -> anyhow::Result<Option<Model>> {
        self.persistence.get_by_id(model_id)
    }

    pub fn delete(&self, model: &Model) -> anyhow::Result<i32> {
        let deleted = self.persistence.delete(model)?;

        let create_model_event = Event::new(model.get_type(), model, EventType::Deleted)?;
        self.stream.send(&create_model_event)?;

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::*;

    use crate::{
        eventing::memory::MemoryEventStream,
        models::{Host, NewHost},
        persistence::memory::MemoryPersistence,
    };

    #[test]
    fn test_create_get_delete() {
        dotenv().ok();

        let new_host = NewHost {
            name: "test".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],

            cpu_capacity: 4000,
            memory_capacity: 24000,
        };

        let host_persistence = MemoryPersistence::<Host, NewHost>::default();

        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let cloned_event_stream = event_stream.clone();
        let host_service =
            Service::<Host, NewHost>::new(Box::new(host_persistence), cloned_event_stream);

        let inserted_host_id = host_service.create(&new_host).unwrap();
        assert_eq!(inserted_host_id, 1);

        let fetched_host = host_service.get_by_id(inserted_host_id).unwrap().unwrap();
        assert_eq!(fetched_host.id, 1);

        let deleted_hosts = host_service.delete(&fetched_host).unwrap();
        assert_eq!(deleted_hosts, 1);
    }
}
