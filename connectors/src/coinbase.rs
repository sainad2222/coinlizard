use crate::ExchangeConnector;
use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use common::{
    models::{CurrentPrice, Exchange, PriceHistory, PriceHistoryPoint, PriceInterval, TradingPair},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

const COINBASE_API_URL: &str = "https://api.coinbase.com/v2";
const COINBASE_PRO_API_URL: &str = "https://api.exchange.coinbase.com";

pub struct CoinbaseConnector {
    client: reqwest::Client,
}

impl CoinbaseConnector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    fn format_product_id(&self, pair: &TradingPair) -> String {
        format!("{}-{}", pair.base, pair.quote)
    }
}

#[derive(Debug, Deserialize)]
struct CoinbaseResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct CoinbaseSpotPrice {
    base: String,
    currency: String,
    amount: String,
}

#[derive(Debug, Deserialize)]
struct CoinbaseCandle {
    time: i64,
    low: String,
    high: String,
    open: String,
    close: String,
    volume: String,
}

#[derive(Debug, Serialize)]
struct CoinbaseHistoricalParams {
    start: Option<String>,
    end: Option<String>,
    granularity: Option<u32>,
}

// Convert PriceInterval to Coinbase granularity (seconds)
fn coinbase_granularity(interval: PriceInterval) -> u32 {
    match interval {
        PriceInterval::OneMinute => 60,
        PriceInterval::FiveMinutes => 300,
        PriceInterval::FifteenMinutes => 900,
        PriceInterval::OneHour => 3600,
        PriceInterval::FourHours => 14400,
        PriceInterval::OneDay => 86400,
        PriceInterval::OneWeek => 604800,
    }
}

#[async_trait]
impl ExchangeConnector for CoinbaseConnector {
    async fn get_current_price(&self, pair: &TradingPair) -> Result<CurrentPrice> {
        let url = format!(
            "{}/prices/{}-{}/spot",
            COINBASE_API_URL, pair.base, pair.quote
        );

        debug!("Fetching current price from Coinbase: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(Error::HttpError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Coinbase API error: {} - {}", status, error_text);
            return Err(Error::ExchangeError(format!(
                "Coinbase API error: {} - {}",
                status, error_text
            )));
        }

        let price_data: CoinbaseResponse<CoinbaseSpotPrice> =
            response.json().await.map_err(|e| {
                Error::ParseError(format!("Failed to parse Coinbase response: {}", e))
            })?;

        let price = price_data
            .data
            .amount
            .parse::<f64>()
            .map_err(|e| Error::ParseError(format!("Failed to parse price: {}", e)))?;

        Ok(CurrentPrice {
            exchange: Exchange::Coinbase,
            pair: pair.clone(),
            price,
            volume_24h: None, // Coinbase spot API doesn't provide volume
            timestamp: Utc::now(),
        })
    }

    async fn get_price_history(
        &self,
        pair: &TradingPair,
        interval: PriceInterval,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<PriceHistory> {
        // Use Coinbase Pro/Exchange API for historical data
        let product_id = self.format_product_id(pair);
        let url = format!("{}/products/{}/candles", COINBASE_PRO_API_URL, product_id);

        // Set default time range if not provided
        let end = end_time.unwrap_or_else(Utc::now);
        // Default to 300 candles worth of data if start time not provided
        let granularity: u32 = coinbase_granularity(interval);
        let start = start_time.unwrap_or_else(|| {
            end - Duration::seconds(granularity as i64 * limit.unwrap_or(300) as i64)
        });

        debug!(
            "Fetching price history from Coinbase: {} (interval: {:?}, start: {}, end: {})",
            url, interval, start, end
        );

        let response = self
            .client
            .get(&url)
            .query(&[
                ("start", start.to_rfc3339()),
                ("end", end.to_rfc3339()),
                ("granularity", granularity.to_string()),
            ])
            .send()
            .await
            .map_err(Error::HttpError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Coinbase API error: {} - {}", status, error_text);
            return Err(Error::ExchangeError(format!(
                "Coinbase API error: {} - {}",
                status, error_text
            )));
        }

        // Coinbase returns an array of arrays: [time, low, high, open, close, volume]
        let candles: Vec<Vec<serde_json::Value>> = response.json().await.map_err(|e| {
            Error::ParseError(format!("Failed to parse Coinbase candles: {}", e))
        })?;

        let mut data_points = Vec::with_capacity(candles.len());

        for candle in candles {
            if candle.len() < 6 {
                continue; // Skip malformed candles
            }

            let timestamp = match candle[0].as_i64() {
                Some(ts) => Utc.timestamp_opt(ts, 0).unwrap(),
                None => continue,
            };

            let close_price = match candle[4].as_str() {
                Some(price_str) => match price_str.parse::<f64>() {
                    Ok(price) => price,
                    Err(_) => continue,
                },
                None => match candle[4].as_f64() {
                    Some(price) => price,
                    None => continue,
                },
            };

            let volume = match candle[5].as_str() {
                Some(vol_str) => match vol_str.parse::<f64>() {
                    Ok(vol) => Some(vol),
                    Err(_) => None,
                },
                None => candle[5].as_f64(),
            };

            data_points.push(PriceHistoryPoint {
                timestamp,
                price: close_price,
                volume,
            });
        }

        // Sort by timestamp (newest first)
        data_points.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Limit results if requested
        if let Some(limit_val) = limit {
            data_points.truncate(limit_val);
        }

        Ok(PriceHistory {
            exchange: Exchange::Coinbase,
            pair: pair.clone(),
            interval,
            data: data_points,
        })
    }

    async fn list_trading_pairs(&self) -> Result<Vec<TradingPair>> {
        let url = format!("{}/products", COINBASE_PRO_API_URL);

        debug!("Fetching trading pairs from Coinbase: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(Error::HttpError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Coinbase API error: {} - {}", status, error_text);
            return Err(Error::ExchangeError(format!(
                "Coinbase API error: {} - {}",
                status, error_text
            )));
        }

        #[derive(Deserialize)]
        struct Product {
            id: String,
            base_currency: String,
            quote_currency: String,
        }

        let products: Vec<Product> = response.json().await.map_err(|e| {
            Error::ParseError(format!("Failed to parse Coinbase products: {}", e))
        })?;

        let pairs = products
            .into_iter()
            .map(|product| TradingPair {
                base: product.base_currency,
                quote: product.quote_currency,
            })
            .collect();

        Ok(pairs)
    }
} 