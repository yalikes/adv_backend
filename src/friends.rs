use std::vec;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::debug;
use tracing_subscriber::field::debug;

use crate::{
    helper::{get_user_id, ConnectionPool, Session, SessionMap},
    user_info::UserInfo,
};

#[derive(Debug, Serialize)]
enum FriendsInfoQueryState {
    Ok,
    Error,
}

#[derive(Debug, Serialize)]
struct FriendsInfo {
    friends: Vec<UserInfo>,
}

#[derive(Debug, Serialize)]
pub struct FriendsInfoResult {
    state: FriendsInfoQueryState,
    info: Option<FriendsInfo>,
}

#[derive(Debug, Deserialize)]
pub struct FriendsInfoRequest {
    session: Session,
}

#[derive(Debug, Deserialize)]
pub struct AddFriendRequest {
    session: Session,
    friend_id: u64,
}

#[derive(Debug, Serialize)]
enum AddFriendState {
    Ok,
    Error,
    AlreadyFriend,
}

#[derive(Debug, Serialize)]
pub struct AddFriendResult {
    state: AddFriendState,
    info: Option<FriendsInfo>,
}

pub async fn query_friends_info(
    State(pool): State<ConnectionPool>,
    State(session_map): State<SessionMap>,
    Json(friends_info_req): Json<FriendsInfoRequest>,
) -> Json<FriendsInfoResult> {
    let session = friends_info_req.session;
    let user_id = get_user_id(session_map, session);
    if user_id.is_none() {
        return FriendsInfoResult {
            state: FriendsInfoQueryState::Error,
            info: None,
        }
        .into();
    }
    let user_id = user_id.unwrap();

    let friends = get_friends(pool, user_id as i64).await;
    let friends = match friends {
        Ok(f) => f,
        Err(e) => {
            debug!("{:?}", e);
            vec![]
        }
    };
    FriendsInfoResult {
        state: FriendsInfoQueryState::Ok,
        info: Some(FriendsInfo { friends: friends }),
    }
    .into()
}

async fn get_friends(pool: ConnectionPool, user_id: i64) -> Result<Vec<UserInfo>, ()> {
    let friend_ids: Vec<i64> = get_friend_ids(&pool, user_id).await?;
    let mut friends: Vec<UserInfo> = vec![];
    for id in friend_ids {
        let row = sqlx::query_as::<_, UserInfo>(
            r#"
            SELECT user_id, user_name, avatar
            FROM adv_chat.user
            WHERE user_id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&pool)
        .await;
        if let Ok(user) = row {
            friends.push(user);
        }
    }
    Ok(friends)
}
async fn get_friend_ids(pool: &ConnectionPool, user_id: i64) -> Result<Vec<i64>, ()> {
    let rows = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT UNNEST(friends)
        FROM adv_chat.user
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ())?;

    let friend_ids: Vec<i64> = rows.iter().map(|i| i.0).collect();
    Ok(friend_ids)
}

async fn set_friend_ids(
    pool: &ConnectionPool,
    user_id: i64,
    friend_ids: Vec<i64>,
) -> Result<(), sqlx::Error> {
    let _result = sqlx::query(
        r#"
        UPDATE adv_chat.user
        SET friends = $1
        WHERE user_id = $2
        "#,
    )
    .bind(&friend_ids)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn user_add_friend(
    State(pool): State<ConnectionPool>,
    State(session_map): State<SessionMap>,
    Json(add_friend_req): Json<AddFriendRequest>,
) -> Json<AddFriendResult> {
    let session = add_friend_req.session;
    let user_id = get_user_id(session_map, session);
    let friend_id = add_friend_req.friend_id as i64;
    debug!("add_friends: {:?}", user_id);
    if user_id.is_none() {
        return AddFriendResult {
            state: AddFriendState::Error,
            info: None,
        }
        .into();
    }
    let user_id = user_id.unwrap();
    let friend_ids = get_friend_ids(&pool, user_id as i64).await;
    let friends_friend_ids = get_friend_ids(&pool, friend_id).await;

    let mut friend_ids = match friend_ids {
        Ok(p) => p,
        Err(_) => {
            debug!("can't read friends");
            return AddFriendResult {
                state: AddFriendState::Error,
                info: None,
            }
            .into();
        }
    };
    let mut friends_friend_ids = match friends_friend_ids {
        Ok(p) => p,
        Err(_) => {
            debug!("can't read friend's friends");
            return AddFriendResult {
                state: AddFriendState::Error,
                info: None,
            }
            .into();
        }
    };

    if !friend_ids.contains(&(add_friend_req.friend_id as i64)) {
        friend_ids.push(add_friend_req.friend_id as i64);
    }

    if !friends_friend_ids.contains(&(user_id as i64)) {
        friends_friend_ids.push(user_id as i64);
    }

    match set_friend_ids(&pool, user_id as i64, friend_ids).await {
        Ok(_) => {}
        Err(e) => {
            debug!("{:?}", e);
            return AddFriendResult {
                state: AddFriendState::Error,
                info: None,
            }
            .into();
        }
    }

    match set_friend_ids(&pool, friend_id, friends_friend_ids).await {
        Ok(_) => {}
        Err(e) => {
            debug!("{:?}", e);
            return AddFriendResult {
                state: AddFriendState::Error,
                info: None,
            }
            .into();
        }
    }

    let friends = match get_friends(pool, user_id as i64).await {
        Ok(f) => f,
        Err(_) => {
            return AddFriendResult {
                state: AddFriendState::Error,
                info: None,
            }
            .into();
        }
    };
    AddFriendResult {
        state: AddFriendState::Ok,
        info: Some(FriendsInfo { friends: friends }),
    }
    .into()
}
