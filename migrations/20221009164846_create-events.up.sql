CREATE TABLE events (
  id                         TEXT      PRIMARY KEY,
  event_timestamp            TIMESTAMP NOT NULL,
  consumer_id                TEXT      NOT NULL,

  operation_id               TEXT      NOT NULL,
  model_type                 INTEGER   NOT NULL,

  serialized_current_model   BYTEA,
  serialized_previous_model  BYTEA,

  event_type                 INTEGER   NOT NULL
);