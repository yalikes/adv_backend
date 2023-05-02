use std::{
    sync::mpsc::{Receiver, RecvError},
    thread,
};
use tokio::task;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::helper::{GroupInfoTable, UserConnectionMap};

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

pub async fn message_private() {}

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
                    let mut connection_guard = user_connection_map.lock().unwrap();
                    // let may_connection = connection_guard.get_mut(&user_id);
                    // if may_connection.is_some() {
                    //     let connection = may_connection.unwrap();
                    //     let send_task = connection
                    //         .send(axum::extract::ws::Message::Text("Hello".to_owned()));
                    //     tokio::spawn(send_task); //this can't compile
                    // }
                }
                MessageType::Group => {}
            },
            Err(e) => {
                debug!("{:?}", e);
            }
        }
    }
}
