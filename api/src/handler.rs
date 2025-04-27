use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use common::{
    models::{Coin, CurrentPrice, Exchange, PriceHistory, PriceInterval},
    Error as CommonError,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};

use crate::service::CoinService;

type SharedService = Arc<RwLock<CoinService>>;

// Create a wrapper for our common::Error type
pub struct ApiError(CommonError);

// Implement From<CommonError> for ApiError
impl From<CommonError> for ApiError {
    fn from(err: CommonError) -> Self {
        ApiError(err)
    }
}

// Convert our API error wrapper to an Axum response
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            CommonError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            CommonError::ExchangeError(msg) => (StatusCode::BAD_GATEWAY, msg),
            CommonError::ParseError(msg) => (StatusCode::BAD_REQUEST, msg),
            CommonError::DbError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            CommonError::HttpError(e) => (
                StatusCode::BAD_GATEWAY,
                format!("External API request failed: {}", e),
            ),
            CommonError::ConfigError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            CommonError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        #[derive(Serialize)]
        struct ErrorResponse {
            error: String,
        }

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

// Return all supported coins
pub async fn list_coins(State(service): State<SharedService>) -> Result<Json<Vec<Coin>>, ApiError> {
    let service = service.read().await;
    let coins = service.list_coins().await?;
    Ok(Json(coins))
}

#[derive(Debug, Deserialize)]
pub struct PriceQuery {
    pub currency: Option<String>,
    pub exchange: Option<String>,
}

// Get current price for a coin
pub async fn get_current_price(
    State(service): State<SharedService>,
    Path(coin_id): Path<String>,
    Query(query): Query<PriceQuery>,
) -> Result<Json<Vec<CurrentPrice>>, ApiError> {
    let service = service.read().await;
    
    // Default to USD if no currency specified
    let currency = query.currency.unwrap_or_else(|| "USD".to_string());
    
    // Parse exchange parameter if provided
    let exchange = match query.exchange.as_deref() {
        Some("coinbase") => Some(Exchange::Coinbase),
        Some("binance") => Some(Exchange::Binance),
        Some(unknown) => {
            return Err(CommonError::ParseError(format!(
                "Unknown exchange: {}. Supported exchanges: coinbase, binance",
                unknown
            )).into())
        }
        None => None,
    };

    let prices = service.get_current_price(&coin_id, &currency, exchange).await?;
    Ok(Json(prices))
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub interval: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

// Get price history for a coin
pub async fn get_price_history(
    State(service): State<SharedService>,
    Path(coin_id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<PriceHistory>, ApiError> {
    let service = service.read().await;
    
    // Default to USD if no currency specified
    let currency = query.currency.unwrap_or_else(|| "USD".to_string());
    
    // Parse exchange parameter if provided
    let exchange = match query.exchange.as_deref() {
        Some("coinbase") => Some(Exchange::Coinbase),
        Some("binance") => Some(Exchange::Binance),
        Some(unknown) => {
            return Err(CommonError::ParseError(format!(
                "Unknown exchange: {}. Supported exchanges: coinbase, binance",
                unknown
            )).into())
        }
        None => None,
    };

    // Parse interval parameter, default to daily
    let interval = match query.interval.as_deref() {
        Some("1m") => PriceInterval::OneMinute,
        Some("5m") => PriceInterval::FiveMinutes,
        Some("15m") => PriceInterval::FifteenMinutes,
        Some("1h") => PriceInterval::OneHour,
        Some("4h") => PriceInterval::FourHours,
        Some("1d") | None => PriceInterval::OneDay,
        Some("1w") => PriceInterval::OneWeek,
        Some(unknown) => {
            return Err(CommonError::ParseError(format!(
                "Unknown interval: {}. Supported intervals: 1m, 5m, 15m, 1h, 4h, 1d, 1w",
                unknown
            )).into())
        }
    };

    let history = service
        .get_price_history(
            &coin_id,
            &currency,
            interval,
            exchange,
            query.start,
            query.end,
            query.limit,
        )
        .await?;

    Ok(Json(history))
} 