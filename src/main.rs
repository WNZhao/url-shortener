mod config;
mod error;
mod handler;
mod model;

use axum::{Router, routing::{delete, get, post}};
use sqlx::mysql::MySqlPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::AppConfig;
use handler::AppState;

#[tokio::main]
async fn main() {
    // init tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "url_shortener=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // load .env
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env();

    // connect to database
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to MySQL");

    tracing::info!("Connected to MySQL");

    let state = AppState {
        db: pool,
        base_url: config.base_url,
    };

    // build router
    let app = Router::new()
        .route("/health", get(handler::health))
        .route("/api/shorten", post(handler::create_short_url))
        .route("/api/stats/{code}", get(handler::get_stats))
        .route("/api/url/{code}", delete(handler::delete_short_url))
        .route("/{code}", get(handler::redirect))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // start server
    let listener = tokio::net::TcpListener::bind(&config.server_addr).await.unwrap();
    tracing::info!("Server listening on {}", config.server_addr);
    axum::serve(listener, app).await.unwrap();
}
