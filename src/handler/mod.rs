use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use chrono::Utc;
use sqlx::MySqlPool;

use crate::model::{CreateUrlRequest, CreateUrlResponse, UrlStatsResponse};

pub async fn health() -> &'static str {
    "OK"
}

pub async fn create_short_url(
    State(pool): State<MySqlPool>,
    Json(payload): Json<CreateUrlRequest>,
) -> Result<Json<CreateUrlResponse>, (StatusCode, String)> {
    let short_code = nanoid::nanoid!(6);

    let expires_at = payload.expires_in_hours.map(|hours| {
        Utc::now().naive_utc() + chrono::Duration::hours(hours)
    });

    sqlx::query(
        "INSERT INTO short_urls (short_code, original_url, expires_at) VALUES (?, ?, ?)",
    )
    .bind(&short_code)
    .bind(&payload.url)
    .bind(expires_at)
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(CreateUrlResponse {
        short_url: format!("http://localhost:3000/{}", short_code),
        short_code,
        original_url: payload.url,
    }))
}

pub async fn redirect(
    State(pool): State<MySqlPool>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let row: Option<(String, Option<chrono::NaiveDateTime>)> = sqlx::query_as(
        "SELECT original_url, expires_at FROM short_urls WHERE short_code = ?",
    )
    .bind(&code)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match row {
        Some((url, expires_at)) => {
            if let Some(exp) = expires_at {
                if Utc::now().naive_utc() > exp {
                    return Err((StatusCode::GONE, "Link has expired".to_string()));
                }
            }
            // increment clicks in background
            let pool = pool.clone();
            let code = code.clone();
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE short_urls SET clicks = clicks + 1 WHERE short_code = ?")
                    .bind(&code)
                    .execute(&pool)
                    .await;
            });

            Ok(Redirect::temporary(&url))
        }
        None => Err((StatusCode::NOT_FOUND, "Short URL not found".to_string())),
    }
}

pub async fn get_stats(
    State(pool): State<MySqlPool>,
    Path(code): Path<String>,
) -> Result<Json<UrlStatsResponse>, (StatusCode, String)> {
    let row: Option<UrlStatsResponse> = sqlx::query_as(
        "SELECT short_code, original_url, clicks, created_at, expires_at FROM short_urls WHERE short_code = ?",
    )
    .bind(&code)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match row {
        Some(stats) => Ok(Json(stats)),
        None => Err((StatusCode::NOT_FOUND, "Short URL not found".to_string())),
    }
}
