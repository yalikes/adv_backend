use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::debug;

use crate::helper::ConnectionPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Group {
    group_id: i64,
    group_name: String,
}

pub async fn get_group(pool: &ConnectionPool, group_id: i64) -> Result<Group, sqlx::Error> {
    let group = sqlx::query_as::<_, Group>(r#"
        SELECT group_id, group_name
        FROM adv_chat.group
        WHERE group_id = $1
        "#)
        .bind(group_id)
        .fetch_one(pool)
        .await?;
    Ok(group)
}