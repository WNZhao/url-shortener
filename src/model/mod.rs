use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ShortUrl {
    pub id: i64,
    pub short_code: String,
    pub original_url: String,
    pub clicks: i64,
    pub created_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUrlRequest {
    pub url: String,
    pub expires_in_hours: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreateUrlResponse {
    pub short_code: String,
    pub short_url: String,
    pub original_url: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UrlStatsResponse {
    pub short_code: String,
    pub original_url: String,
    pub clicks: i64,
    pub created_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
}
