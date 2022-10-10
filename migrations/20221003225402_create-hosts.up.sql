CREATE TABLE hosts (
  id                TEXT        PRIMARY KEY,

  labels            TEXT[]      NOT NULL
);

CREATE INDEX hosts_labels on hosts USING GIN(labels);