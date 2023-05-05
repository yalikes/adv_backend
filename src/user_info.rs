use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::debug;

use crate::{
    group_info::{get_group, Group},
    helper::{get_user_id, ConnectionPool, Session, SessionMap},
};

#[derive(Debug, Serialize)]
enum UserInfoQueryState {
    Ok,
    Error,
}

#[derive(Debug, Serialize, FromRow)]
pub struct UserInfo {
    user_id: i64,
    user_name: String,
    avatar: String,
}

#[derive(Debug, Serialize)]
pub struct UserInfoResult {
    state: UserInfoQueryState,
    info: Option<UserInfo>,
}

#[derive(Debug, Deserialize)]
pub struct UserInfoRequest {
    user_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct ThisUserRequest {
    session: Session,
}

pub async fn query_user_info(
    State(pool): State<ConnectionPool>,
    Json(user_info_req): Json<UserInfoRequest>,
) -> Json<UserInfoResult> {
    let user_id = user_info_req.user_id;
    let (user_id, user_name, avatar): (i64, String, String) = match sqlx::query_as(
        "
    SELECT user_id, user_name, avatar
    FROM adv_chat.user
    WHERE user_id = $1
    ",
    )
    .bind(user_id as i64)
    .fetch_one(&pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            debug!("can't find user: {:?}", e);
            return UserInfoResult {
                state: UserInfoQueryState::Error,
                info: None,
            }
            .into();
        }
    };
    UserInfoResult {
        state: UserInfoQueryState::Ok,
        info: Some(UserInfo {
            user_id: user_id,
            user_name,
            avatar,
        }),
    }
    .into()
}

pub async fn query_user_this(
    State(pool): State<ConnectionPool>,
    State(session_map): State<SessionMap>,
    Json(user_info_req): Json<ThisUserRequest>,
) -> Json<UserInfoResult> {
    let session = user_info_req.session;
    let user_id = get_user_id(session_map, session);
    if user_id.is_none() {
        return UserInfoResult {
            state: UserInfoQueryState::Error,
            info: None,
        }
        .into();
    }
    let user_id = user_id.unwrap();
    let (user_id, user_name, avatar): (i64, String, String) = match sqlx::query_as(
        "
    SELECT user_id, user_name, avatar
    FROM adv_chat.user
    WHERE user_id = $1
    ",
    )
    .bind(user_id as i64)
    .fetch_one(&pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            debug!("can't find user: {:?}", e);
            return UserInfoResult {
                state: UserInfoQueryState::Error,
                info: None,
            }
            .into();
        }
    };
    UserInfoResult {
        state: UserInfoQueryState::Ok,
        info: Some(UserInfo {
            user_id: user_id,
            user_name,
            avatar,
        }),
    }
    .into()
}

#[derive(Debug, Serialize)]
pub struct UserGroupsResult {
    state: UserInfoQueryState,
    groups: Option<Vec<Group>>,
}

#[derive(Debug, Deserialize)]
pub struct UserGroupsRequest {
    session: Session,
}

pub async fn query_user_groups(
    State(pool): State<ConnectionPool>,
    State(session_map): State<SessionMap>,
    Json(user_groups_req): Json<UserGroupsRequest>,
) -> Json<UserGroupsResult> {
    let session = user_groups_req.session;
    let user_id = get_user_id(session_map, session);
    if user_id.is_none() {
        return UserGroupsResult {
            state: UserInfoQueryState::Error,
            groups: None,
        }
        .into();
    }
    let user_id = user_id.unwrap();
    let group_ids = match get_user_group_ids(&pool, user_id as i64).await {
        Ok(g_ids) => g_ids,
        Err(e) => {
            debug!("at query group ids: {:?}", e);
            return UserGroupsResult {
                state: UserInfoQueryState::Error,
                groups: None,
            }
            .into();
        }
    };
    let mut groups = vec![];
    for g_id in group_ids {
        let g = get_group(&pool, g_id).await;
        if g.is_ok() {
            let g = g.unwrap();
            groups.push(g);
        }
    }
    UserGroupsResult {
        state: UserInfoQueryState::Ok,
        groups: Some(groups),
    }
    .into()
}

async fn get_user_group_ids(pool: &ConnectionPool, user_id: i64) -> Result<Vec<i64>, sqlx::Error> {
    let group_list = sqlx::query_as::<_, (i64,)>(
        r#"
    SELECT UNNEST(group_list)
    FROM adv_chat.user
    WHERE user_id = $1
    "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let group_list = group_list.iter().map(|g_id| g_id.0).collect();
    Ok(group_list)
}
