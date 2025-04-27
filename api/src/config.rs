use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
        }
    }
}

impl ApiConfig {
    pub fn from_env() -> Self {
        let host = std::env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("API_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        Self { host, port }
    }
} 