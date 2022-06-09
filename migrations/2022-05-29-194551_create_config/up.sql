CREATE TABLE configs (
  id         TEXT      PRIMARY KEY,

  model_type SMALLINT  NOT NULL,
  model_id   TEXT      NOT NULL,

  key        TEXT      NOT NULL,
  value      TEXT      NOT NULL
);