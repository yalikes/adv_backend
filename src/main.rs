use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service, post},
    Router,
};
use hyper::http::HeaderValue;
use hyper::Method;
use std::net::SocketAddr;
use std::{collections::HashMap, env};
use tower_http::cors::{AllowOrigin, Cors};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber;
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
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                return "hello";
            }),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(
                    ["http://frontend.org", "http://frontend.org:5173"]
                        .map(|x| x.parse::<HeaderValue>().unwrap()),
                ))
                .allow_methods([Method::GET, Method::POST]),
        );
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
