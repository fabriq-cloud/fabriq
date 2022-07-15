CREATE TABLE workloads (
  id           TEXT PRIMARY KEY,
  name         TEXT NOT NULL,
  
  team_id      TEXT NOT NULL,
  template_id  TEXT NOT NULL REFERENCES templates(id)
);