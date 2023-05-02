use axum::extract::ws::WebSocket;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use sqlx::{pool::Pool, Postgres};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

pub type SessionMap = Arc<Mutex<LruCache<Session, u64>>>;
pub type ConnectionPool = Pool<Postgres>;
pub type GroupInfoTable = Arc<Mutex<LruCache<u64, Vec<u64>>>>;
pub type UserConnectionMap = Arc<Mutex<HashMap<u64, WebSocket>>>;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Session {
    pub session_id: Uuid,
}
