use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::{OffsetDateTime, UtcOffset, PrimitiveDateTime};
use tracing::debug;

use crate::{
    helper::{get_user_id, ConnectionPool, MessageSender, OperationState, Session, SessionMap},
    message::{ChatMessage, ChatMessageStored},
    user_info::{get_user_group_ids, query_user_groups, ThisUserRequest},
};

#[derive(Debug, Serialize)]
pub struct SyncMessagesResult {
    state: OperationState,
    messages: Option<Vec<ChatMessage>>,
}

#[derive(Debug, Deserialize)]
pub struct SyncMessagesRequest {
    session: Session,
    days: u64,
}

pub async fn sync_message_client(
    State(pool): State<ConnectionPool>,
    State(session_map): State<SessionMap>,
    Json(sync_messages_req): Json<SyncMessagesRequest>,
) -> Json<SyncMessagesResult> {
    let session = sync_messages_req.session;
    let user_id = get_user_id(session_map, session);
    if user_id.is_none() {
        return SyncMessagesResult {
            state: OperationState::Err,
            messages: None,
        }
        .into();
    }
    let user_id = user_id.unwrap();
    let messages = read_messages(&pool, user_id as i64).await.unwrap();
    SyncMessagesResult {
        state: OperationState::Ok,
        messages: Some(messages),
    }
    .into()
}

async fn read_messages(
    pool: &ConnectionPool,
    user_id: i64,
) -> Result<Vec<ChatMessage>, sqlx::Error> {
    let mut private_messages = sqlx::query_as::<_, ChatMessageStored>(
        r#"
        SELECT 
        message as content,
        message_from as sender_id,
        message_to as receiver_id,
        created_at as time
        FROM adv_chat.private_message
        WHERE message_from = $1 OR message_to = $1
    "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut private_messages: Vec<ChatMessage> = private_messages
        .iter()
        .map(|m| ChatMessage {
            content: m.content.clone(),
            message_type: crate::message::MessageType::Private,
            sender_id: m.sender_id as u64,
            receiver_id: m.receiver_id as u64,
            time: m.time,
        })
        .collect();

    let group_ids = get_user_group_ids(pool, user_id).await.unwrap();
    let mut group_msgs: Vec<ChatMessageStored> = vec![];
    for g_id in group_ids {
        let mut g_messages = sqlx::query_as::<_, ChatMessageStored>(
            r#"
                SELECT 
                group_message as content,
                message_from as sender_id,
                group_id as receiver_id,
                created_at as time
                FROM adv_chat.group_message
                WHERE group_id = $1
                "#,
        )
        .bind(g_id)
        .fetch_all(pool)
        .await?;
        group_msgs.append(&mut g_messages);
    }
    let mut group_msgs: Vec<ChatMessage> = group_msgs
        .iter()
        .map(|m| ChatMessage {
            content: m.content.clone(),
            message_type: crate::message::MessageType::Group,
            sender_id: m.sender_id as u64,
            receiver_id: m.receiver_id as u64,
            time: m.time,
        })
        .collect();
    group_msgs.append(&mut private_messages);
    group_msgs.sort_by(|a, b| PrimitiveDateTime::cmp(&a.time, &b.time));
    Ok(group_msgs)
}
