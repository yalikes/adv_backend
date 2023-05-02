use axum::extract::FromRef;

use crate::helper::ConnectionPool;
use crate::helper::GroupInfoTable;
use crate::helper::SessionMap;
use crate::helper::UserConnectionMap;

#[derive(Clone)]
pub struct AppState {
    pub sesson_map: SessionMap,
    pub db_pool: ConnectionPool,
    pub group_info_table: GroupInfoTable,
    pub user_connection_map: UserConnectionMap,
}

impl FromRef<AppState> for SessionMap {
    fn from_ref(input: &AppState) -> Self {
        input.sesson_map.clone()
    }
}

impl FromRef<AppState> for ConnectionPool {
    fn from_ref(input: &AppState) -> Self {
        input.db_pool.clone()
    }
}
impl FromRef<AppState> for GroupInfoTable {
    fn from_ref(input: &AppState) -> Self {
        input.group_info_table.clone()
    }
}

impl FromRef<AppState> for UserConnectionMap {
    fn from_ref(input: &AppState) -> Self {
        input.user_connection_map.clone()
    }
}