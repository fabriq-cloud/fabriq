use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection,
};

use lazy_static::lazy_static;
use std::env;

lazy_static! {
    pub static ref POOL: Pool<ConnectionManager<PgConnection>> = Pool::builder()
        .max_size(8)
        .build(ConnectionManager::<PgConnection>::new(
            env::var("EVENTS_DATABASE_URL").expect("EVENTS_DATABASE_URL must be set"),
        ))
        .expect("Failed to create DB connection pool");
}

pub fn get_connection() -> anyhow::Result<PooledConnection<ConnectionManager<PgConnection>>> {
    Ok(POOL.clone().get()?)
}
