use axum::extract::FromRef;

use crate::helper::SessionMap;
use crate::helper::ConnectionPool;

#[derive(Clone)]
pub struct AppState{
    pub sesson_map:  SessionMap,
    pub db_pool: ConnectionPool,
}

impl FromRef<AppState> for SessionMap {
    fn from_ref(input: &AppState) -> Self {
        input.sesson_map.clone()
    }
}

impl  FromRef<AppState> for ConnectionPool {
    fn from_ref(input: &AppState) -> Self {
        input.db_pool.clone()
    }
}