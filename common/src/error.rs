use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    DbError(String),

    #[error("Exchange API error: {0}")]
    ExchangeError(String),

    #[error("Parsing error: {0}")]
    ParseError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
} 