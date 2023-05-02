use app_state::AppState;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    routing::post,
    Json, Router,
};
use dotenvy::dotenv;
use futures::stream::SplitStream;
use hyper::http::HeaderValue;
use hyper::Method;
use lru::LruCache;
use message::{message_private, message_processing};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use user_info::query_user_info;
use std::collections::HashMap;
use std::env;
use std::num::NonZeroUsize;
use std::sync::mpsc;
use std::thread;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use futures::{sink::SinkExt, stream::StreamExt};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::debug;
use tracing_subscriber::{self};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utils::generate_salt_and_hash;
use uuid::Uuid;

mod app_state;
mod helper;
mod message;
mod utils;
mod user_info;

use helper::{ConnectionPool, GroupInfoTable, Session, SessionMap, UserConnectionMap};

use crate::utils::check;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must set");

    let session_cache: SessionMap = Arc::new(Mutex::new(LruCache::new(
        NonZeroUsize::new(1024 * 1024).unwrap(),
    )));

    let group_info_table: GroupInfoTable = Arc::new(Mutex::new(LruCache::new(
        NonZeroUsize::new(64 * 1024).unwrap(),
    )));
    let user_connection_map: UserConnectionMap = Arc::new(Mutex::new(HashMap::new()));
    let (message_sender, message_receiver) = mpsc::channel();
    let message_sender = Arc::new(Mutex::new(message_sender));

    let group_info_table_ref = group_info_table.clone();
    let user_connection_map_ref = user_connection_map.clone();
    thread::spawn(move || {
        message_processing(
            message_receiver,
            user_connection_map_ref,
            group_info_table_ref,
        )
    });
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = PgPoolOptions::new()
        .max_connections(32)
        .connect(&database_url)
        .await
        .expect("failed to connect database");
    let state = AppState {
        sesson_map: session_cache,
        db_pool: pool.clone(),
        group_info_table: group_info_table.clone(),
        user_connection_map: user_connection_map.clone(),
        message_sender: message_sender,
    };
    thread::spawn(move || {});
    let app = Router::new()
        .route("/user/register", post(user_register))
        .route("/user/login", post(user_login))
        .route("/user/info", post(query_user_info))
        .route("/tunnel", get(ws_handler))
        .route("/message/private", post(message_private))
        .route("/message/group", post(|| async {}))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(
                    ["http://frontend.org", "http://frontend.org:5173"]
                        .map(|x| x.parse::<HeaderValue>().unwrap()),
                ))
                .allow_methods([Method::GET, Method::POST])
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

#[derive(Debug, Serialize)]
enum UserRegisterState {
    Success,
    PasswordTooWeak,
    OtherError,
}

#[derive(Debug, Serialize)]
struct UserRegisterResultInfo {
    state: UserRegisterState,
    session_info: Option<Session>,
    user_id: Option<u64>,
}

#[derive(Debug, Serialize)]
enum UserLoginState {
    Success,
    WrongPassword,
    OtherError,
}

#[derive(Debug, Serialize)]
struct UserLoginInfo {
    state: UserLoginState,
    session_info: Option<Session>,
}

#[derive(Debug, Deserialize)]
struct UserLoginRequest {
    user_id: u64,
    password: String,
}

#[derive(Debug, Deserialize)]
struct UserRegisterRequest {
    username: String,
    password: String,
}

async fn user_register(
    State(pool): State<ConnectionPool>,
    State(sesson_map): State<SessionMap>,
    Json(user_reg_req): Json<UserRegisterRequest>,
) -> Json<UserRegisterResultInfo> {
    if !check_password(&user_reg_req.password) {
        return UserRegisterResultInfo {
            state: UserRegisterState::PasswordTooWeak,
            session_info: None,
            user_id: None,
        }
        .into();
    }
    if !check_username(&user_reg_req.username) {
        return UserRegisterResultInfo {
            state: UserRegisterState::OtherError,
            session_info: None,
            user_id: None,
        }
        .into();
    }

    let (hash, salt) = generate_salt_and_hash(&user_reg_req.password);
    let salt: String = salt.iter().collect();

    let uuid = Uuid::new_v4();
    let session_id = Session { session_id: uuid };
    let user_id: i64 = match sqlx::query_as::<_, (i64,)>(
        "INSERT INTO adv_chat.user
        (user_name, user_passwd_hash, salt, avatar, created_at)
        VALUES($1, $2, $3, $4, now() at time zone 'utc') 
        RETURNING user_id",
    )
    .bind(&user_reg_req.username)
    .bind(hash)
    .bind(salt)
    .bind("#27A5F3")
    .fetch_one(&pool)
    .await
    {
        Ok(r) => r.0,
        Err(_) => {
            return UserRegisterResultInfo {
                state: UserRegisterState::OtherError,
                session_info: None,
                user_id: None,
            }
            .into();
        }
    };
    sesson_map.lock().unwrap().put(session_id, user_id as u64);
    UserRegisterResultInfo {
        state: UserRegisterState::Success,
        session_info: Some(session_id),
        user_id: Some(user_id as u64),
    }
    .into()
}

fn check_password(password: &str) -> bool {
    if password.len() < 6 {
        return false;
    }
    true
}

fn check_username(username: &str) -> bool {
    if username.is_empty() {
        return false;
    }
    true
}

async fn user_login(
    State(pool): State<ConnectionPool>,
    State(sesson_map): State<SessionMap>,
    Json(user_login_req): Json<UserLoginRequest>,
) -> Json<UserLoginInfo> {
    if !check_password(&user_login_req.password) {
        return UserLoginInfo {
            state: UserLoginState::WrongPassword,
            session_info: None,
        }
        .into();
    }
    let user_id = user_login_req.user_id;
    let password = user_login_req.password;
    let (user_passwd_hash, salt): ([u8; 32], String) = match sqlx::query_as(
        "
    SELECT user_passwd_hash, salt
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
            return UserLoginInfo {
                state: UserLoginState::OtherError,
                session_info: None,
            }
            .into();
        }
    };
    if check(&password, &salt, user_passwd_hash) {
        let uuid = Uuid::new_v4();
        let session_id = Session { session_id: uuid };
        sesson_map.lock().unwrap().put(session_id, user_id as u64);
        UserLoginInfo {
            state: UserLoginState::Success,
            session_info: Some(session_id),
        }
        .into()
    } else {
        UserLoginInfo {
            state: UserLoginState::WrongPassword,
            session_info: None,
        }
        .into()
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(sesson_map): State<SessionMap>,
    State(user_connection_map): State<UserConnectionMap>,
) -> impl IntoResponse {
    debug!("{:?}", ws);
    ws.on_upgrade(move |socket| handle_socket(socket, sesson_map, user_connection_map))
}
async fn handle_socket(
    socket: WebSocket,
    session_map: SessionMap,
    user_connection_map: UserConnectionMap,
) {
    let (mut sender, mut receiver) = socket.split();
    let (sender_unbouned, mut receiver_unbounded) =
        tokio::sync::mpsc::unbounded_channel::<Message>();
    let session: Session = match check_token(&mut receiver).await {
        Ok(s) => s,
        Err(_) => {
            return;
        }
    };
    debug!("{:?}", session);
    let user_id = match session_map.lock().unwrap().get(&session) {
        Some(user_id) => *user_id,
        None => {
            return;
        }
    };
    user_connection_map
        .lock()
        .unwrap()
        .insert(user_id, sender_unbouned);
    while let Some(r) = receiver_unbounded.recv().await {
        sender.send(r).await;
    }
}

async fn check_token(stream: &mut SplitStream<WebSocket>) -> Result<Session, ()> {
    if let Some(Ok(m)) = stream.next().await {
        if let Message::Text(t) = m {
            serde_json::from_str(&t).map_err(|e| debug!("{:?}", e))
        } else {
            Err(())
        }
    } else {
        Err(())
    }
}


