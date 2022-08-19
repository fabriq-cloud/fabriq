#[macro_use]
extern crate diesel;

mod db;
mod event;
mod postgresql;
mod schema;

pub use postgresql::PostgresqlEventStream;
