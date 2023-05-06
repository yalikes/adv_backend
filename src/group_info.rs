use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::debug;

use crate::helper::{get_user_id, ConnectionPool, Session, SessionMap};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Group {
    group_id: i64,
    group_name: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupNewRequest {
    session: Session,
    group_name: String,
}

#[derive(Debug, Serialize)]
pub enum GroupNewState {
    Ok,
    NotLogin,
    TooShortGroupName,
    OtherError,
}

#[derive(Debug, Serialize)]
pub struct GroupNewRespone {
    state: GroupNewState,
    group_id: Option<u64>,
}

pub async fn get_group(pool: &ConnectionPool, group_id: i64) -> Result<Group, sqlx::Error> {
    let group = sqlx::query_as::<_, Group>(
        r#"
        SELECT group_id, group_name
        FROM adv_chat.group
        WHERE group_id = $1
        "#,
    )
    .bind(group_id)
    .fetch_one(pool)
    .await?;
    Ok(group)
}

pub async fn new_group(
    State(pool): State<ConnectionPool>,
    State(session_map): State<SessionMap>,
    Json(group_new_req): Json<GroupNewRequest>,
) -> Json<GroupNewRespone> {
    let session = group_new_req.session;
    let user_id = get_user_id(session_map, session);
    if user_id.is_none() {
        return GroupNewRespone {
            state: GroupNewState::NotLogin,
            group_id: None,
        }
        .into();
    }
    let user_id = user_id.unwrap();
    if group_new_req.group_name.len() < 1 {
        return GroupNewRespone {
            state: GroupNewState::TooShortGroupName,
            group_id: None,
        }
        .into();
    }

    let group_id = sqlx::query_as::<_, (i64,)>(
        r#"INSERT INTO adv_chat.group
        (group_name, group_host, user_list, created_at)
        VALUES($1, $2, $3, now() at time zone 'utc')
        RETURNING group_id"#,
    )
    .bind(group_new_req.group_name)
    .bind(user_id as i64)
    .bind(vec![user_id as i64])
    .fetch_one(&pool)
    .await;
    let group_id = match group_id {
        Ok(g_id) => g_id.0,
        Err(e) => {
            debug!("failed to exec sql: {:?}", e);
            return GroupNewRespone {
                state: GroupNewState::OtherError,
                group_id: None,
            }
            .into();
        }
    };
    GroupNewRespone {
        state: GroupNewState::Ok,
        group_id: Some(group_id as u64),
    }
    .into()
}

pub async fn get_group_users(pool: &ConnectionPool, group_id: i64) -> Result<Vec<i64>, sqlx::Error> {
    let g_users = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT UNNEST(user_list)
        FROM adv_chat.group
        WHERE group_id = $1
    "#,
    )
    .bind(group_id)
    .fetch_all(pool)
    .await?;
    let g_users: Vec<i64> = g_users.iter().map(|u| u.0).collect();
    return Ok(g_users);
}

pub async fn set_group_users(
    pool: &ConnectionPool,
    group_id: i64,
    new_user_ids: &[i64],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE adv_chat.group
        SET user_list = $1
        WHERE group_id = $2
    "#,
    )
    .bind(new_user_ids)
    .bind(group_id)
    .fetch_all(pool)
    .await?;
    return Ok(());
}

pub async fn group_add_user(
    pool: &ConnectionPool,
    group_id: i64,
    new_user_id: i64,
) -> Result<Vec<i64>, sqlx::Error> {
    let mut g_user_ids = get_group_users(pool, group_id).await?;
    if g_user_ids.contains(&new_user_id){
        return Ok(g_user_ids);
    }
    g_user_ids.push(new_user_id);
    set_group_users(pool, group_id, &g_user_ids).await?;
    return Ok(g_user_ids)
}