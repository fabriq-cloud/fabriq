CREATE TABLE configs (
  id           TEXT PRIMARY KEY,

  owning_model TEXT NOT NULL,

  key          TEXT NOT NULL,
  value        TEXT NOT NULL
);

CREATE INDEX configs_owning_model ON configs(owning_model);