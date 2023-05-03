use axum::{extract::State, Json};
use futures::{sink::SinkExt, stream::StreamExt};
use std::{
    sync::mpsc::{Receiver, RecvError},
    thread,
};
use tokio::task;
use tracing_subscriber::field::debug;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    helper::{
        ConnectionPool, GroupInfoTable, MessageSender, Session, SessionMap, UserConnectionMap,
    },
    UserLoginRequest,
};

#[derive(Debug, Serialize)]
pub struct MessagePlain {
    user_id: u64,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageType {
    Private,
    Group,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub message_type: MessageType,
    pub content: String,
    pub sender_id: u64,
    pub reciver_id: u64,
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

pub async fn message_private(
    State(pool): State<ConnectionPool>,
    State(sesson_map): State<SessionMap>,
    State(message_sender): State<MessageSender>,
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
    message_sender
        .lock()
        .unwrap()
        .send(ChatMessage {
            message_type: message_req.message_type,
            content: message_req.content,
            sender_id: user_id,
            reciver_id: message_req.reciver_id,
        })
        .unwrap();
    ChatMessageInfo {
        state: ChatMessageInfoState::Ok,
    }
    .into()
}

pub fn message_processing(
    receiver: Receiver<ChatMessage>,
    user_connection_map: UserConnectionMap,
    group_info_table: GroupInfoTable,
) {
    loop {
        match receiver.recv() {
            Ok(msg) => match msg.message_type {
                MessageType::Private => {
                    let user_id = msg.reciver_id;
                    debug!("{:?}", msg);
                    if let Some(may_sender) = user_connection_map.lock().unwrap().get_mut(&user_id)
                    {
                        if let Err(err) = may_sender.send(axum::extract::ws::Message::Text(
                            serde_json::to_string(&MessagePlain {
                                user_id,
                                content: msg.content,
                            })
                            .unwrap(),
                        )) {
                            debug!("{:?}", err);
                        }
                    }
                }
                MessageType::Group => {}
            },
            Err(e) => {
                debug!("{:?}", e);
            }
        }
    }
}
