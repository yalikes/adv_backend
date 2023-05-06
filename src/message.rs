use axum::{extract::State, Json};
use futures::{sink::SinkExt, stream::StreamExt};
use sqlx::FromRow;
use std::{
    sync::mpsc::{Receiver, RecvError},
    thread,
};
use tokio::task;
use tracing_subscriber::field::debug;

use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};
use tracing::debug;

use crate::{
    group_info::{get_group_users, get_group_users_sync},
    helper::{
        ConnectionPool, GroupInfoTable, MessageSender, Session, SessionMap, UserConnectionMap,
    },
    UserLoginRequest,
};

#[derive(Debug, Serialize)]
pub struct MessagePlain {
    message_type: MessageType,
    user_id: u64,
    group_id: Option<u64>,
    content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    Private,
    Group,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub message_type: MessageType,
    pub content: String,
    pub sender_id: u64,
    pub receiver_id: u64,
    pub time: PrimitiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ChatMessageStored {
    pub content: String,
    pub sender_id: i64,
    pub receiver_id: i64,
    pub time: PrimitiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessageRequest {
    pub message_type: MessageType,
    pub content: String,
    pub reciver_id: u64,
    pub seesion: Session,
}

#[derive(Debug, Serialize)]
enum ChatMessageInfoState {
    Ok,
    WrongToken,
    OtherError,
}

#[derive(Debug, Serialize)]
pub struct ChatMessageInfo {
    state: ChatMessageInfoState,
}

pub async fn message_from_client(
    State(sesson_map): State<SessionMap>,
    State(message_sender): State<MessageSender>,
    State(pool): State<ConnectionPool>,
    Json(message_req): Json<ChatMessageRequest>,
) -> Json<ChatMessageInfo> {
    let user_id = match sesson_map.lock().unwrap().get(&message_req.seesion) {
        Some(user_id) => *user_id,
        None => {
            return ChatMessageInfo {
                state: ChatMessageInfoState::WrongToken,
            }
            .into();
        }
    };
    let now = OffsetDateTime::now_utc();
    let message = ChatMessage {
        message_type: message_req.message_type,
        content: message_req.content,
        sender_id: user_id,
        receiver_id: message_req.reciver_id,
        time: PrimitiveDateTime::new(now.date(), now.time()),
    };
    record_message(&pool, message.clone()).await.unwrap();
    message_sender.lock().unwrap().send(message).unwrap();
    ChatMessageInfo {
        state: ChatMessageInfoState::Ok,
    }
    .into()
}

pub fn message_processing(
    pool: ConnectionPool,
    receiver: Receiver<ChatMessage>,
    user_connection_map: UserConnectionMap,
) {
    loop {
        match receiver.recv() {
            Ok(msg) => match msg.message_type {
                MessageType::Private => {
                    let user_id = msg.receiver_id;
                    let sender_id = msg.sender_id;
                    debug!("{:?}", msg);
                    if let Some(may_sender) = user_connection_map.lock().unwrap().get_mut(&user_id)
                    {
                        if let Err(err) = may_sender.send(axum::extract::ws::Message::Text(
                            serde_json::to_string(&MessagePlain {
                                message_type: MessageType::Private,
                                user_id: sender_id,
                                group_id: None,
                                content: msg.content,
                            })
                            .unwrap(),
                        )) {
                            debug!("{:?}", err);
                        }
                    }
                }
                MessageType::Group => {
                    let group_id = msg.receiver_id;
                    let sender_id = msg.sender_id;
                    let group_user_ids = match get_group_users_sync(&pool, group_id as i64) {
                        Ok(g) => g,
                        Err(e) => {
                            debug!("{:?}", e);
                            vec![]
                        }
                    };
                    debug!("{:?}", group_user_ids);
                    for uid in group_user_ids {
                        if let Some(may_sender) =
                            user_connection_map.lock().unwrap().get_mut(&(uid as u64))
                        {
                            if let Err(err) = may_sender.send(axum::extract::ws::Message::Text(
                                serde_json::to_string(&MessagePlain {
                                    message_type: MessageType::Group,
                                    user_id: sender_id,
                                    group_id: Some(group_id),
                                    content: msg.content.clone(),
                                })
                                .unwrap(),
                            )) {
                                debug!("{:?}", err);
                            }
                        }
                    }
                }
            },
            Err(e) => {
                debug!("{:?}", e);
            }
        }
    }
}

async fn record_message(pool: &ConnectionPool, message: ChatMessage) -> Result<(), sqlx::Error> {
    match message.message_type {
        MessageType::Group => {
            sqlx::query(
                r#"
                INSERT INTO adv_chat.group_message
                (message_from, group_id, group_message, created_at)
                VALUES($1, $2, $3, $4)
            "#,
            )
            .bind(message.sender_id as i64)
            .bind(message.receiver_id as i64)
            .bind(message.content)
            .bind(message.time)
            .execute(pool)
            .await?;
        }
        MessageType::Private => {
            sqlx::query(
                r#"
            INSERT INTO adv_chat.private_message
            (message_from, message_to, message, created_at)
            VALUES($1, $2, $3, $4::timestamp)
        "#,
            )
            .bind(message.sender_id as i64)
            .bind(message.receiver_id as i64)
            .bind(message.content)
            .bind(message.time)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
