use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::debug;

use crate::helper::{get_user_id, ConnectionPool, Session, SessionMap};

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
    session: Session,
}

pub async fn query_user_info(
    State(pool): State<ConnectionPool>,
    State(session_map): State<SessionMap>,
    Json(user_info_req): Json<UserInfoRequest>,
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
