use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service, post},
    Router, Json,
};
use dotenvy::dotenv;
use hyper::Method;
use hyper::{header, http::HeaderValue, HeaderMap};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::{collections::HashMap, env};
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

#[tokio::main]
async fn main() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must set");
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
        .route(
            "/",
            get(|| async {
                return "hello";
            }),
        )
        .route("/testapi", get(test_api))
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

#[derive(Debug, Serialize, Deserialize)]
struct Session {
    session_id: String,
}

async fn test_api() -> impl IntoResponse {
    let session_id = Session {
        session_id: "1".to_owned(),
    };
    // CORS cookie 不好搞, 用JSON吧
    Json::from(session_id)
}
