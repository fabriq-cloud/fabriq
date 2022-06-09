CREATE TABLE workloads (
  id           TEXT PRIMARY KEY,
  
  workspace_id TEXT NOT NULL REFERENCES workspaces(id),
  template_id  TEXT NOT NULL REFERENCES templates(id)
);