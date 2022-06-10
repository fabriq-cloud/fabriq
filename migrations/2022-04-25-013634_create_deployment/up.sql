CREATE TABLE deployments (
  id           TEXT    PRIMARY KEY,
  
  workload_id  TEXT    NOT NULL REFERENCES workloads(id),
  target_id    TEXT    NOT NULL REFERENCES targets(id),
  
  hosts        INTEGER NOT NULL
);