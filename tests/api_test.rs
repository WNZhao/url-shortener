use axum::http::StatusCode;
use axum::{
    Router,
    body::Body,
    routing::{delete, get, post},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::mysql::MySqlPoolOptions;
use tower::ServiceExt;

use url_shortener::handler::{self, AppState};

async fn setup() -> Router {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to MySQL");

    // clean test data
    sqlx::query("DELETE FROM short_urls WHERE short_code LIKE 'test_%'")
        .execute(&pool)
        .await
        .unwrap();

    let state = AppState {
        db: pool,
        base_url: "http://localhost:3030".to_string(),
    };

    Router::new()
        .route("/health", get(handler::health))
        .route("/api/shorten", post(handler::create_short_url))
        .route("/api/stats/{code}", get(handler::get_stats))
        .route("/api/url/{code}", delete(handler::delete_short_url))
        .route("/{code}", get(handler::redirect))
        .with_state(state)
}

fn json_request(method: &str, uri: &str, body: Option<Value>) -> axum::http::Request<Body> {
    let builder = axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json");

    match body {
        Some(b) => builder.body(Body::from(b.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

async fn response_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_health() {
    let app = setup().await;
    let resp = app
        .oneshot(json_request("GET", "/health", None))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], b"OK");
}

#[tokio::test]
async fn test_create_and_stats() {
    let app = setup().await;

    // create with custom code
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/shorten",
            Some(json!({"url": "https://rust-lang.org", "custom_code": "test_rs"})),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["short_code"], "test_rs");
    assert_eq!(body["original_url"], "https://rust-lang.org");

    // get stats
    let resp = app
        .clone()
        .oneshot(json_request("GET", "/api/stats/test_rs", None))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["short_code"], "test_rs");
    assert_eq!(body["clicks"], 0);
}

#[tokio::test]
async fn test_redirect() {
    let app = setup().await;

    // create
    app.clone()
        .oneshot(json_request(
            "POST",
            "/api/shorten",
            Some(json!({"url": "https://example.com", "custom_code": "test_rdr"})),
        ))
        .await
        .unwrap();

    // redirect
    let resp = app
        .clone()
        .oneshot(json_request("GET", "/test_rdr", None))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        resp.headers().get("location").unwrap().to_str().unwrap(),
        "https://example.com"
    );
}

#[tokio::test]
async fn test_delete() {
    let app = setup().await;

    // create
    app.clone()
        .oneshot(json_request(
            "POST",
            "/api/shorten",
            Some(json!({"url": "https://delete-me.com", "custom_code": "test_del"})),
        ))
        .await
        .unwrap();

    // delete
    let resp = app
        .clone()
        .oneshot(json_request("DELETE", "/api/url/test_del", None))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // verify gone
    let resp = app
        .clone()
        .oneshot(json_request("GET", "/api/stats/test_del", None))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_custom_code_validation() {
    let app = setup().await;

    // too short
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/shorten",
            Some(json!({"url": "https://x.com", "custom_code": "ab"})),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // invalid chars
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/shorten",
            Some(json!({"url": "https://x.com", "custom_code": "a b c"})),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_not_found() {
    let app = setup().await;

    let resp = app
        .oneshot(json_request("GET", "/api/stats/nonexist999", None))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 404);
}
