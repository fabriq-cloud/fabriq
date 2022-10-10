CREATE TABLE templates (
  id         TEXT  PRIMARY KEY,

  repository TEXT  NOT NULL,
  git_ref    TEXT  NOT NULL,
  path       TEXT  NOT NULL
);