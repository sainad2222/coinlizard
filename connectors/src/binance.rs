use crate::ExchangeConnector;
use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use common::{
    models::{CurrentPrice, Exchange, PriceHistory, PriceHistoryPoint, PriceInterval, TradingPair},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

const BINANCE_API_URL: &str = "https://api.binance.com/api/v3";

pub struct BinanceConnector {
    client: reqwest::Client,
}

impl BinanceConnector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    fn format_symbol(&self, pair: &TradingPair) -> String {
        format!("{}{}", pair.base, pair.quote)
    }
}

#[derive(Debug, Deserialize)]
struct BinanceTickerPrice {
    symbol: String,
    price: String,
}

#[derive(Debug, Deserialize)]
struct Binance24hTicker {
    symbol: String,
    #[serde(rename = "lastPrice")]
    last_price: String,
    #[serde(rename = "volume")]
    volume: String,
}

// Convert PriceInterval to Binance interval string
fn binance_interval(interval: PriceInterval) -> &'static str {
    match interval {
        PriceInterval::OneMinute => "1m",
        PriceInterval::FiveMinutes => "5m",
        PriceInterval::FifteenMinutes => "15m",
        PriceInterval::OneHour => "1h",
        PriceInterval::FourHours => "4h",
        PriceInterval::OneDay => "1d",
        PriceInterval::OneWeek => "1w",
    }
}

#[async_trait]
impl ExchangeConnector for BinanceConnector {
    async fn get_current_price(&self, pair: &TradingPair) -> Result<CurrentPrice> {
        let symbol = self.format_symbol(pair);
        let url = format!("{}/ticker/24hr", BINANCE_API_URL);

        debug!("Fetching 24hr ticker from Binance for {}", symbol);

        let response = self
            .client
            .get(&url)
            .query(&[("symbol", &symbol)])
            .send()
            .await
            .map_err(Error::HttpError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Binance API error: {} - {}", status, error_text);
            return Err(Error::ExchangeError(format!(
                "Binance API error: {} - {}",
                status, error_text
            )));
        }

        let ticker: Binance24hTicker = response.json().await.map_err(|e| {
            Error::ParseError(format!("Failed to parse Binance response: {}", e))
        })?;

        let price = ticker
            .last_price
            .parse::<f64>()
            .map_err(|e| Error::ParseError(format!("Failed to parse price: {}", e)))?;

        let volume = ticker
            .volume
            .parse::<f64>()
            .ok()
            .map(|v| v * price); // Convert to quote currency volume

        Ok(CurrentPrice {
            exchange: Exchange::Binance,
            pair: pair.clone(),
            price,
            volume_24h: volume,
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
        let symbol = self.format_symbol(pair);
        let url = format!("{}/klines", BINANCE_API_URL);

        let now = Utc::now();
        let end = end_time.unwrap_or(now);
        
        // Default to 1000 candles (Binance limit) if start time not provided
        let binance_limit = limit.unwrap_or(1000).min(1000); // Binance max limit is 1000
        
        let mut params = vec![
            ("symbol", symbol),
            ("interval", binance_interval(interval).to_string()),
            ("limit", binance_limit.to_string()),
        ];

        if let Some(start) = start_time {
            // Binance uses millisecond timestamps
            params.push(("startTime", start.timestamp_millis().to_string()));
        }

        if let Some(end) = end_time {
            // Binance uses millisecond timestamps
            params.push(("endTime", end.timestamp_millis().to_string()));
        }

        debug!(
            "Fetching price history from Binance: {} (interval: {:?}, limit: {})",
            url, interval, binance_limit
        );

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(Error::HttpError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Binance API error: {} - {}", status, error_text);
            return Err(Error::ExchangeError(format!(
                "Binance API error: {} - {}",
                status, error_text
            )));
        }

        // Binance returns an array of arrays:
        // [
        //   [
        //     1499040000000,      // Open time
        //     "0.01634790",       // Open
        //     "0.80000000",       // High
        //     "0.01575800",       // Low
        //     "0.01577100",       // Close
        //     "148976.11427815",  // Volume
        //     ...                 // (more fields we don't need)
        //   ]
        // ]
        let candles: Vec<Vec<serde_json::Value>> = response.json().await.map_err(|e| {
            Error::ParseError(format!("Failed to parse Binance candles: {}", e))
        })?;

        let mut data_points = Vec::with_capacity(candles.len());

        for candle in candles {
            if candle.len() < 6 {
                continue; // Skip malformed candles
            }

            let timestamp = match candle[0].as_i64() {
                Some(ts) => Utc.timestamp_millis_opt(ts).unwrap(),
                None => continue,
            };

            let close_price = match candle[4].as_str() {
                Some(price_str) => match price_str.parse::<f64>() {
                    Ok(price) => price,
                    Err(_) => continue,
                },
                None => continue,
            };

            let volume = match candle[5].as_str() {
                Some(vol_str) => match vol_str.parse::<f64>() {
                    Ok(vol) => Some(vol),
                    Err(_) => None,
                },
                None => None,
            };

            data_points.push(PriceHistoryPoint {
                timestamp,
                price: close_price,
                volume,
            });
        }

        // Sort by timestamp (newest first)
        data_points.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(PriceHistory {
            exchange: Exchange::Binance,
            pair: pair.clone(),
            interval,
            data: data_points,
        })
    }

    async fn list_trading_pairs(&self) -> Result<Vec<TradingPair>> {
        let url = format!("{}/exchangeInfo", BINANCE_API_URL);

        debug!("Fetching exchange info from Binance: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(Error::HttpError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Binance API error: {} - {}", status, error_text);
            return Err(Error::ExchangeError(format!(
                "Binance API error: {} - {}",
                status, error_text
            )));
        }

        #[derive(Deserialize)]
        struct Symbol {
            symbol: String,
            baseAsset: String,
            quoteAsset: String,
        }

        #[derive(Deserialize)]
        struct ExchangeInfo {
            symbols: Vec<Symbol>,
        }

        let info: ExchangeInfo = response.json().await.map_err(|e| {
            Error::ParseError(format!("Failed to parse Binance exchange info: {}", e))
        })?;

        let pairs = info
            .symbols
            .into_iter()
            .map(|symbol| TradingPair {
                base: symbol.baseAsset,
                quote: symbol.quoteAsset,
            })
            .collect();

        Ok(pairs)
    }
} 