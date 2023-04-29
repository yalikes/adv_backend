use serde::{Deserialize, Serialize};
use uuid::Uuid;
use lru::LruCache;
use std::sync::{Arc, Mutex};
use sqlx::{pool::Pool, Postgres};

pub type SessionMap = Arc<Mutex<LruCache<Session, u64>>>;
pub type ConnectionPool = Pool<Postgres>;
#[derive(Debug,Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Session {
    pub session_id: Uuid,
}
