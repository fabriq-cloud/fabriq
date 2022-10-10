use async_trait::async_trait;
use chrono::NaiveDateTime;
use sqlx::PgPool;
use std::{sync::Arc, time::SystemTime};

use akira_core::{Event, EventStream};

use crate::model::PostgreSQLEvent;

#[derive(Debug)]
pub struct PostgresqlEventStream {
    pub db: Arc<PgPool>,
    pub subscribers: Vec<String>,
}

impl PostgresqlEventStream {
    async fn upsert(&self, event: &PostgreSQLEvent) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            r#"
                INSERT INTO events
                    (id,

                     event_timestamp,
                     consumer_id,
                     operation_id,
                     model_type,

                     serialized_current_model,
                     serialized_previous_model,

                     event_type)
                VALUES
                    ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (id) DO NOTHING
                "#,
            event.id,
            event.event_timestamp,
            event.consumer_id,
            event.operation_id,
            event.model_type,
            event.serialized_current_model,
            event.serialized_previous_model,
            event.event_type
        )
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }

    async fn upsert_many(&self, events: &[PostgreSQLEvent]) -> anyhow::Result<u64> {
        let mut total_rows_affected = 0;

        for event in events {
            let rows_affected = self.upsert(event).await?;
            total_rows_affected += rows_affected;
        }

        Ok(total_rows_affected)
    }

    #[allow(dead_code)]
    async fn clear(&self) -> anyhow::Result<u64> {
        let result = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM events
            "#
        )
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected())
    }
}

#[async_trait]
impl EventStream for PostgresqlEventStream {
    async fn send(&self, event: &Event) -> anyhow::Result<()> {
        if event.operation_id.is_none() {
            return Err(anyhow::anyhow!("operation_id not provided"));
        }

        let operation_id = event.operation_id.as_ref().unwrap();

        let models: Vec<PostgreSQLEvent> = self
            .subscribers
            .iter()
            .map(|consumer_id| PostgreSQLEvent {
                id: PostgreSQLEvent::make_id(&operation_id.id, consumer_id),
                event_timestamp: NaiveDateTime::from_timestamp(
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    0,
                ),
                consumer_id: consumer_id.to_string(),

                operation_id: operation_id.id.clone(),

                model_type: event.model_type,
                event_type: event.event_type,

                serialized_current_model: event.serialized_current_model.clone(),
                serialized_previous_model: event.serialized_previous_model.clone(),
            })
            .collect();

        self.upsert_many(&models).await?;

        Ok(())
    }

    async fn send_many(&self, events: &[Event]) -> anyhow::Result<()> {
        for event in events {
            self.send(event).await?;
        }

        Ok(())
    }

    async fn delete(&self, event: &Event, consumer_id: &str) -> anyhow::Result<u64> {
        if event.operation_id.is_none() {
            return Err(anyhow::anyhow!("operation_id is not supported"));
        }

        let operation_id = event.operation_id.as_ref().unwrap();

        let event_id = PostgreSQLEvent::make_id(&operation_id.id, consumer_id);

        let result = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM events WHERE id = $1
            "#,
            event_id
        )
        .execute(&*self.db)
        .await?;

        let test = result.rows_affected();
        Ok(test)
    }

    async fn receive(&self, consumer_id: &str) -> anyhow::Result<Vec<Event>> {
        let rows = sqlx::query_as!(
            PostgreSQLEvent,
            r#"
                SELECT * FROM events WHERE consumer_id = $1
            "#,
            consumer_id
        )
        .fetch_all(&*self.db)
        .await?;

        let models = rows.into_iter().map(Event::from).collect::<Vec<Event>>();

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use prost::Message;
    use prost_types::Timestamp;
    use sqlx::postgres::PgPoolOptions;
    use std::time::SystemTime;

    use akira_core::{Event, EventType, HostMessage, ModelType, OperationId};

    use super::*;

    #[tokio::test]
    async fn test_send_create_host_event() {
        const RECONCILER_CONSUMER_ID: &str = "reconciler";
        const GITOPS_CONSUMER_ID: &str = "gitops";

        dotenvy::from_filename(".env.test").ok();

        let host = HostMessage {
            id: "azure-eastus2-1".to_owned(),
            labels: vec!["location:eastus2".to_string(), "cloud:azure".to_string()],
        };

        let database_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let db = Arc::new(
            PgPoolOptions::new()
                .max_connections(1)
                .connect(&database_url)
                .await
                .expect("failed to connect to DATABASE_URL"),
        );

        let event_stream = PostgresqlEventStream {
            db,
            subscribers: vec![
                RECONCILER_CONSUMER_ID.to_string(),
                GITOPS_CONSUMER_ID.to_string(),
            ],
        };

        event_stream.clear().await.unwrap();

        let operation_id = OperationId::create();

        let timestamp = Timestamp {
            seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        };

        let create_host_event = Event {
            operation_id: Some(operation_id),
            model_type: ModelType::Host as i32,
            serialized_current_model: Some(host.encode_to_vec()),
            serialized_previous_model: None,
            event_type: EventType::Created as i32,
            timestamp: Some(timestamp),
        };

        event_stream.send(&create_host_event).await.unwrap();

        // receive event back on the reconciler consumer

        let received_events = event_stream.receive(RECONCILER_CONSUMER_ID).await.unwrap();

        assert_eq!(received_events.len(), 1);

        let received_event = received_events.first().unwrap();

        assert_eq!(received_event.event_type, EventType::Created as i32);
        assert_eq!(received_event.model_type, ModelType::Host as i32);

        let decoded_host: HostMessage = HostMessage::decode(
            received_event
                .serialized_current_model
                .as_ref()
                .unwrap()
                .as_slice(),
        )
        .unwrap();

        assert_eq!(decoded_host.id, host.id);

        // since we haven't deleted the event, we should still receive it

        let received_events = event_stream.receive(RECONCILER_CONSUMER_ID).await.unwrap();
        assert_eq!(received_events.len(), 1);

        event_stream
            .delete(received_event, RECONCILER_CONSUMER_ID)
            .await
            .unwrap();

        // test that we have deleted and can't now fetch event for the reconciler consumer

        let received_events = event_stream.receive(RECONCILER_CONSUMER_ID).await.unwrap();
        assert_eq!(received_events.len(), 0);

        let received_events = event_stream.receive(GITOPS_CONSUMER_ID).await.unwrap();

        // test that there is still can receive the event for the gitops consumer

        println!("{:?}", received_events);
        assert_eq!(received_events.len(), 1);

        assert_eq!(received_event.event_type, EventType::Created as i32);
        assert_eq!(received_event.model_type, ModelType::Host as i32);
    }
}
