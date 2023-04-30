use app_state::AppState;
use axum::{
    async_trait,
    extract::{FromRequest, State},
    http::StatusCode,
    response::{IntoResponse, IntoResponseParts, Response},
    routing::{get, get_service, post},
    Json, Router,
};
use dotenvy::dotenv;
use hyper::{
    body::HttpBody,
    header::{self, ACCEPT},
    http::HeaderValue,
    HeaderMap,
};
use hyper::{Method, Request};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use utils::generate_salt_and_hash;
use std::env;
use std::num::NonZeroUsize;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use std::{thread, time::Duration};
use tower_http::cors::{AllowOrigin, Cors};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{debug, info};
use tracing_subscriber::{self};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::{uuid, Uuid};

mod app_state;
mod helper;
mod utils;

use helper::{ConnectionPool, Session, SessionMap};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must set");

    let mut session_cache: SessionMap = Arc::new(Mutex::new(LruCache::new(
        NonZeroUsize::new(1024 * 1024).unwrap(),
    )));

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
    let mut state = AppState {
        sesson_map: session_cache,
        db_pool: pool,
    };
    let app = Router::new()
        .route("/user/register", post(user_register))
        .route("/user/login", post(user_login))
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
        .serve(app.into_make_service())
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
}

#[derive(Debug, Serialize)]
struct UserLoginInfo {
    state: UserLoginState,
    session_info: Option<Session>,
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

    let (hash,salt) = generate_salt_and_hash(&user_reg_req.password);
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

fn gen_password_hash() {}

async fn user_login() -> Json<UserLoginInfo> {
    unimplemented!();
}
