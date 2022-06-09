CREATE TABLE templates (
  id         TEXT  PRIMARY KEY,
  
  repository TEXT  NOT NULL,
  branch     TEXT  NOT NULL,
  path       TEXT  NOT NULL
);