use chrono::{DateTime, Utc};
use common::{
    models::{Coin, CurrentPrice, Exchange, PriceHistory, PriceInterval, TradingPair},
    Error, Result,
};
use connectors::ExchangeConnector;
use std::collections::HashMap;
use std::sync::Arc;
use store::{PriceQuery, PriceStore};
use tracing::{debug, error, info};

/// Service for managing coin data and interacting with exchanges
pub struct CoinService {
    /// Coinbase API connector
    coinbase: Arc<dyn ExchangeConnector>,
    /// Binance API connector
    binance: Arc<dyn ExchangeConnector>,
    /// InfluxDB store for price data
    store: Arc<PriceStore>,
    /// Cache of available coins
    coins: HashMap<String, Coin>,
}

impl CoinService {
    pub fn new(
        coinbase: Arc<dyn ExchangeConnector>,
        binance: Arc<dyn ExchangeConnector>,
        store: Arc<PriceStore>,
    ) -> Self {
        // Initialize with some popular coins
        let mut coins = HashMap::new();
        
        let popular_coins = vec![
            Coin {
                id: "bitcoin".to_string(),
                name: "Bitcoin".to_string(),
                symbol: "BTC".to_string(),
            },
            Coin {
                id: "ethereum".to_string(),
                name: "Ethereum".to_string(),
                symbol: "ETH".to_string(),
            },
            Coin {
                id: "ripple".to_string(),
                name: "XRP".to_string(),
                symbol: "XRP".to_string(),
            },
            Coin {
                id: "cardano".to_string(),
                name: "Cardano".to_string(),
                symbol: "ADA".to_string(),
            },
            Coin {
                id: "solana".to_string(),
                name: "Solana".to_string(),
                symbol: "SOL".to_string(),
            },
        ];

        for coin in popular_coins {
            coins.insert(coin.id.clone(), coin);
        }

        Self {
            coinbase,
            binance,
            store,
            coins,
        }
    }

    /// List all available coins
    pub async fn list_coins(&self) -> Result<Vec<Coin>> {
        Ok(self.coins.values().cloned().collect())
    }

    /// Get coin by ID
    pub fn get_coin(&self, id: &str) -> Result<Coin> {
        self.coins
            .get(id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Coin with ID '{}' not found", id)))
    }

    /// Get current price for a coin
    pub async fn get_current_price(
        &self,
        coin_id: &str,
        quote_currency: &str,
        exchange: Option<Exchange>,
    ) -> Result<Vec<CurrentPrice>> {
        let coin = self.get_coin(coin_id)?;
        
        let pair = TradingPair {
            base: coin.symbol.clone(),
            quote: quote_currency.to_uppercase(),
        };

        debug!(
            "Getting current price for {} ({}/{})",
            coin_id, pair.base, pair.quote
        );

        // Try to get price from store first
        match self.store.get_current_price(&pair, exchange).await {
            Ok(prices) if !prices.is_empty() => {
                debug!("Retrieved prices from store: {} results", prices.len());
                return Ok(prices);
            }
            _ => {
                debug!("No prices found in store, fetching from exchanges");
            }
        }

        // Fetch prices from exchanges
        let mut prices = Vec::new();

        // If a specific exchange is requested, only query that one
        if let Some(ex) = exchange {
            match ex {
                Exchange::Coinbase => {
                    match self.coinbase.get_current_price(&pair).await {
                        Ok(price) => {
                            // Store the price for future queries
                            let _ = self.store.store_current_price(&price).await;
                            prices.push(price);
                        }
                        Err(e) => {
                            error!("Failed to get Coinbase price: {}", e);
                        }
                    }
                }
                Exchange::Binance => {
                    match self.binance.get_current_price(&pair).await {
                        Ok(price) => {
                            // Store the price for future queries
                            let _ = self.store.store_current_price(&price).await;
                            prices.push(price);
                        }
                        Err(e) => {
                            error!("Failed to get Binance price: {}", e);
                        }
                    }
                }
            }
        } else {
            // Try both exchanges
            
            // Coinbase
            match self.coinbase.get_current_price(&pair).await {
                Ok(price) => {
                    // Store the price for future queries
                    let _ = self.store.store_current_price(&price).await;
                    prices.push(price);
                }
                Err(e) => {
                    error!("Failed to get Coinbase price: {}", e);
                }
            }
            
            // Binance
            match self.binance.get_current_price(&pair).await {
                Ok(price) => {
                    // Store the price for future queries
                    let _ = self.store.store_current_price(&price).await;
                    prices.push(price);
                }
                Err(e) => {
                    error!("Failed to get Binance price: {}", e);
                }
            }
        }

        if prices.is_empty() {
            return Err(Error::ExchangeError(format!(
                "Failed to get current price for {}/{}",
                pair.base, pair.quote
            )));
        }

        Ok(prices)
    }

    /// Get historical price data for a coin
    pub async fn get_price_history(
        &self,
        coin_id: &str,
        quote_currency: &str,
        interval: PriceInterval,
        exchange: Option<Exchange>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<PriceHistory> {
        let coin = self.get_coin(coin_id)?;
        
        let pair = TradingPair {
            base: coin.symbol.clone(),
            quote: quote_currency.to_uppercase(),
        };

        debug!(
            "Getting price history for {} ({}/{}) with interval {:?}",
            coin_id, pair.base, pair.quote, interval
        );

        // Try to get history from store first
        let query = PriceQuery {
            pair: pair.clone(),
            exchange,
            interval,
            start_time,
            end_time,
            limit,
        };

        match self.store.get_price_history(&query).await {
            Ok(history) if !history.data.is_empty() => {
                debug!("Retrieved history from store: {} points", history.data.len());
                return Ok(history);
            }
            _ => {
                debug!("No history found in store, fetching from exchanges");
            }
        }

        // Fetch history from exchange(s)
        let history = match exchange {
            Some(Exchange::Coinbase) => {
                self.coinbase
                    .get_price_history(&pair, interval, start_time, end_time, limit)
                    .await?
            }
            Some(Exchange::Binance) => {
                self.binance
                    .get_price_history(&pair, interval, start_time, end_time, limit)
                    .await?
            }
            None => {
                // Try Coinbase first, then Binance if Coinbase fails
                match self
                    .coinbase
                    .get_price_history(&pair, interval, start_time, end_time, limit)
                    .await
                {
                    Ok(history) => history,
                    Err(_) => {
                        self.binance
                            .get_price_history(&pair, interval, start_time, end_time, limit)
                            .await?
                    }
                }
            }
        };

        // Store the history for future queries
        let _ = self.store.store_price_history(&history).await;

        Ok(history)
    }
} 