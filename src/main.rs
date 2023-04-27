use axum::{
    http::StatusCode,
    response::{IntoResponse, IntoResponseParts},
    routing::{get, get_service, post},
    Json, Router,
};
use dotenvy::dotenv;
use hyper::Method;
use hyper::{header, http::HeaderValue, HeaderMap};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, env};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use std::{thread, time::Duration};
use tower_http::cors::{AllowOrigin, Cors};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{debug, info};
use tracing_subscriber::{self};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use lru::LruCache;
use std::num::NonZeroUsize;
use uuid::{uuid, Uuid};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must set");

    let mut session_cache: LruCache<Session, String> = LruCache::new(NonZeroUsize::new(1024*1024).unwrap());

    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect database");
    let app = Router::new()
        .route("/user_register", post(user_register))
        .route("/user_login", post(user_login))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(
                    ["http://frontend.org", "http://frontend.org:5173"]
                        .map(|x| x.parse::<HeaderValue>().unwrap()),
                ))
                .allow_methods([Method::GET, Method::POST]),
        )
        .layer(TraceLayer::new_for_http());
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

type SessionMap = Arc<Mutex<HashMap<Session, u64>>>;

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
struct Session {
    session_id: Uuid,
}

#[derive(Debug, Serialize)]
enum UserRegisterState {
    Success,
    NameUsed,
    PasswordTooWeak,
}

#[derive(Debug, Serialize)]
struct UserRegisterResultInfo {
    state: UserRegisterState,
    session_info: Option<Session>,
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

async fn user_register() -> Json<UserRegisterResultInfo> {
    unimplemented!();
}

async fn user_login() -> Json<UserLoginInfo> {
    unimplemented!();
}
