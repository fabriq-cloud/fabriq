CREATE TABLE assignments (
  id            TEXT PRIMARY KEY,

  deployment_id TEXT NOT NULL REFERENCES deployments(id),
  host_id       TEXT NOT NULL REFERENCES hosts(id)
);

CREATE INDEX assignments_deployment_id ON assignments(deployment_id);