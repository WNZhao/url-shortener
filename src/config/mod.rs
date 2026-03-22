use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub server_addr: String,
    pub base_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        let base_url = env::var("BASE_URL")
            .unwrap_or_else(|_| format!("http://localhost:{}", server_addr.split(':').last().unwrap_or("3000")));
        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            server_addr,
            base_url,
        }
    }
}
