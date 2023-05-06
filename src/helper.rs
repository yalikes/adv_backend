use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use sqlx::{pool::Pool, Postgres};
use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc, Mutex},
};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::message::ChatMessage;

pub type SessionMap = Arc<Mutex<LruCache<Session, u64>>>;
pub type ConnectionPool = Pool<Postgres>;
pub type GroupInfoTable = Arc<Mutex<LruCache<u64, Vec<u64>>>>;
pub type UserConnectionMap = Arc<Mutex<HashMap<u64, UnboundedSender<Message>>>>;
pub type MessageSender = Arc<Mutex<Sender<ChatMessage>>>;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Session {
    pub session_id: Uuid,
}

pub fn get_user_id(session_map: SessionMap, session: Session) -> Option<u64> {
    session_map.lock().unwrap().get(&session).map(|u| *u)
}

#[derive(Debug, Serialize)]
pub enum OperationState {
    Ok,
    Err,
}
