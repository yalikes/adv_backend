use serde::{Deserialize, Serialize};
use uuid::Uuid;
use axum::extract::FromRef;
use lru::LruCache;
use std::sync::Arc;
use sqlx::{pool::Pool, Postgres};

pub type SessionMap = Arc<LruCache<Session, u64>>;
pub type ConnectionPool = Pool<Postgres>;
#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Session {
    pub session_id: Uuid,
}
