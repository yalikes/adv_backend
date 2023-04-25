use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service, post},
    Router,
};
use std::{thread, time::Duration};
use hyper::{http::HeaderValue, HeaderMap, header};
use hyper::Method;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::{collections::HashMap, env};
use tower_http::cors::{AllowOrigin, Cors};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{info, debug};
use tracing_subscriber::{self};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use dotenvy::dotenv;

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
        .await.expect("failed to connect database");
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                return "hello";
            }),)
        .route("/testcookie", get(test_cookie))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(
                    ["http://frontend.org", "http://frontend.org:5173"]
                        .map(|x| x.parse::<HeaderValue>().unwrap()),
                ))
                .allow_methods([Method::GET, Method::POST])
        )
        .layer(TraceLayer::new_for_http());
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn test_cookie() -> impl IntoResponse{
    // CORS cookie 不好搞, 用JSON吧
    let mut header = HeaderMap::new();
    let header_val = HeaderValue::from_str("text/html; charset=utf-8").unwrap();
    debug!("{:?}",header_val);
    header.insert(
        header::CONTENT_TYPE,
        header_val,
    );
    header.insert(header::SET_COOKIE, HeaderValue::from_str("A=12").unwrap());
    thread::sleep(Duration::from_secs(5));
    (StatusCode::OK, header, "hello world!".to_owned())
}
