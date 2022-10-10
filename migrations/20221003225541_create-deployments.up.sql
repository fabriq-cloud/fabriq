CREATE TABLE deployments (
  id           TEXT    PRIMARY KEY,
  name         TEXT    NOT NULL,
  workload_id  TEXT    NOT NULL REFERENCES workloads(id),
  target_id    TEXT    NOT NULL REFERENCES targets(id),
  template_id  TEXT             REFERENCES templates(id),

  host_count   INTEGER NOT NULL
);

CREATE INDEX deployments_target_id ON deployments(target_id);
CREATE INDEX deployments_template_id ON deployments(template_id);
CREATE INDEX deployments_workload_id ON deployments(workload_id);