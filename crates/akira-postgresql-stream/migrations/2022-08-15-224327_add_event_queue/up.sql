CREATE TABLE event_queue (
  id                         TEXT      PRIMARY KEY,
  event_timestamp            TIMESTAMP NOT NULL,
  consumer_id                TEXT      NOT NULL,

  operation_id               TEXT,
  model_type                 INTEGER   NOT NULL,

  serialized_current_model   BYTEA,
  serialized_previous_model  BYTEA,

  event_type                 INTEGER   NOT NULL
);

-- CREATE INDEX event_queue_event_timestamp_idx ON event_queue(event_timestamp);
