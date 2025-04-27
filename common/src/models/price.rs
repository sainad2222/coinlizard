use crate::models::{Exchange, TradingPair};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Current price data from an exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentPrice {
    /// The exchange this price is from
    pub exchange: Exchange,
    /// Trading pair (e.g., BTC/USD)
    pub pair: TradingPair,
    /// Current price value
    pub price: f64,
    /// 24h volume in quote currency
    pub volume_24h: Option<f64>,
    /// Timestamp when this price was recorded
    pub timestamp: DateTime<Utc>,
}

/// Price history point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistoryPoint {
    /// Timestamp for this price point
    pub timestamp: DateTime<Utc>,
    /// The price at this point in time
    pub price: f64,
    /// Trading volume for this time period
    pub volume: Option<f64>,
}

/// Historical price data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistory {
    /// The exchange this price history is from
    pub exchange: Exchange,
    /// Trading pair (e.g., BTC/USD)
    pub pair: TradingPair,
    /// Time interval for this price history
    pub interval: PriceInterval,
    /// Price data points
    pub data: Vec<PriceHistoryPoint>,
}

/// Supported time intervals for price history
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PriceInterval {
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "4h")]
    FourHours,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "1w")]
    OneWeek,
}

impl std::fmt::Display for PriceInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PriceInterval::OneMinute => write!(f, "1m"),
            PriceInterval::FiveMinutes => write!(f, "5m"),
            PriceInterval::FifteenMinutes => write!(f, "15m"),
            PriceInterval::OneHour => write!(f, "1h"),
            PriceInterval::FourHours => write!(f, "4h"),
            PriceInterval::OneDay => write!(f, "1d"),
            PriceInterval::OneWeek => write!(f, "1w"),
        }
    }
} 