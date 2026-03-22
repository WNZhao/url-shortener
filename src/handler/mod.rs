use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use chrono::Utc;
use sqlx::MySqlPool;

use crate::model::{CreateUrlRequest, CreateUrlResponse, MessageResponse, UrlStatsResponse};

#[derive(Clone)]
pub struct AppState {
    pub db: MySqlPool,
    pub base_url: String,
}

pub async fn health() -> &'static str {
    "OK"
}

pub async fn create_short_url(
    State(state): State<AppState>,
    Json(payload): Json<CreateUrlRequest>,
) -> Result<Json<CreateUrlResponse>, (StatusCode, String)> {
    let short_code = match &payload.custom_code {
        Some(code) => {
            if code.len() < 3 || code.len() > 10 {
                return Err((StatusCode::BAD_REQUEST, "Custom code must be 3-10 characters".to_string()));
            }
            if !code.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                return Err((StatusCode::BAD_REQUEST, "Custom code can only contain alphanumeric, - and _".to_string()));
            }
            code.clone()
        }
        None => nanoid::nanoid!(6),
    };

    let expires_at = payload.expires_in_hours.map(|hours| {
        Utc::now().naive_utc() + chrono::Duration::hours(hours)
    });

    sqlx::query(
        "INSERT INTO short_urls (short_code, original_url, expires_at) VALUES (?, ?, ?)",
    )
    .bind(&short_code)
    .bind(&payload.url)
    .bind(expires_at)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(CreateUrlResponse {
        short_url: format!("{}/{}", state.base_url, short_code),
        short_code,
        original_url: payload.url,
    }))
}

pub async fn redirect(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let row: Option<(String, Option<chrono::NaiveDateTime>)> = sqlx::query_as(
        "SELECT original_url, expires_at FROM short_urls WHERE short_code = ?",
    )
    .bind(&code)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match row {
        Some((url, expires_at)) => {
            if let Some(exp) = expires_at {
                if Utc::now().naive_utc() > exp {
                    return Err((StatusCode::GONE, "Link has expired".to_string()));
                }
            }
            let db = state.db.clone();
            let code = code.clone();
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE short_urls SET clicks = clicks + 1 WHERE short_code = ?")
                    .bind(&code)
                    .execute(&db)
                    .await;
            });

            Ok(Redirect::temporary(&url))
        }
        None => Err((StatusCode::NOT_FOUND, "Short URL not found".to_string())),
    }
}

pub async fn get_stats(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<UrlStatsResponse>, (StatusCode, String)> {
    let row: Option<UrlStatsResponse> = sqlx::query_as(
        "SELECT short_code, original_url, clicks, created_at, expires_at FROM short_urls WHERE short_code = ?",
    )
    .bind(&code)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match row {
        Some(stats) => Ok(Json(stats)),
        None => Err((StatusCode::NOT_FOUND, "Short URL not found".to_string())),
    }
}

pub async fn delete_short_url(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<MessageResponse>, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM short_urls WHERE short_code = ?")
        .bind(&code)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Short URL not found".to_string()));
    }

    Ok(Json(MessageResponse {
        message: format!("Short URL '{}' deleted", code),
    }))
}
