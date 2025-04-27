pub mod binance;
pub mod coinbase;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common::{
    models::{CurrentPrice, PriceHistory, PriceInterval, TradingPair},
    Result,
};

/// Trait defining the interface for exchange API clients
#[async_trait]
pub trait ExchangeConnector: Send + Sync {
    /// Get the current price for a trading pair
    async fn get_current_price(&self, pair: &TradingPair) -> Result<CurrentPrice>;

    /// Get price history for a trading pair within a specified time range
    async fn get_price_history(
        &self,
        pair: &TradingPair,
        interval: PriceInterval,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<PriceHistory>;

    /// List supported trading pairs
    async fn list_trading_pairs(&self) -> Result<Vec<TradingPair>>;
} 