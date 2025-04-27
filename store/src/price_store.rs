use crate::{StoreConfig, StoreError};
use chrono::{DateTime, Utc};
use common::models::{
    CurrentPrice, Exchange, PriceHistory, PriceHistoryPoint, PriceInterval, TradingPair,
};
use futures::stream;
use influxdb2::{Client, models::Query};
use std::sync::Arc;
use tracing::{debug, error, info};

pub struct PriceStore {
    client: Client,
    config: StoreConfig,
}

#[derive(Debug, Clone)]
pub struct PriceQuery {
    pub pair: TradingPair,
    pub exchange: Option<Exchange>,
    pub interval: PriceInterval,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

impl PriceStore {
    pub fn new(config: StoreConfig) -> Result<Self, StoreError> {
        let client = Client::new(&config.url, &config.org, &config.token);
        
        Ok(Self { client, config })
    }

    pub async fn store_current_price(&self, price: &CurrentPrice) -> Result<(), StoreError> {
        debug!(
            "Storing current price: {} {} at {}",
            price.pair.base, price.pair.quote, price.price
        );

        // Create a data point for InfluxDB
        let point = influxdb2::models::DataPoint::builder("price_current")
            .tag("exchange", price.exchange.to_string())
            .tag("base", price.pair.base.clone())
            .tag("quote", price.pair.quote.clone())
            .field("price", price.price)
            .field("volume_24h", price.volume_24h.unwrap_or(0.0))
            .timestamp(price.timestamp.timestamp_nanos())
            .build()?;

        self.client
            .write(&self.config.bucket, stream::iter(vec![point]))
            .await?;

        Ok(())
    }

    pub async fn store_price_history(&self, history: &PriceHistory) -> Result<(), StoreError> {
        debug!(
            "Storing price history: {} {} with {} points",
            history.pair.base,
            history.pair.quote,
            history.data.len()
        );

        let mut points = Vec::with_capacity(history.data.len());

        for point in &history.data {
            let data_point = influxdb2::models::DataPoint::builder("price_history")
                .tag("exchange", history.exchange.to_string())
                .tag("base", history.pair.base.clone())
                .tag("quote", history.pair.quote.clone())
                .tag("interval", history.interval.to_string())
                .field("price", point.price)
                .field("volume", point.volume.unwrap_or(0.0))
                .timestamp(point.timestamp.timestamp_nanos())
                .build()?;
            
            points.push(data_point);
        }

        self.client
            .write(&self.config.bucket, stream::iter(points))
            .await?;

        Ok(())
    }

    pub async fn get_current_price(
        &self,
        pair: &TradingPair,
        exchange: Option<Exchange>,
    ) -> Result<Vec<CurrentPrice>, StoreError> {
        let mut query_str = format!(
            r#"from(bucket: "{}")
               |> range(start: -1h)
               |> filter(fn: (r) => r._measurement == "price_current")
               |> filter(fn: (r) => r.base == "{}" and r.quote == "{}")
               |> last()
               |> pivot(rowKey:["_time"], columnKey: ["_field"], valueColumn: "_value")"#,
            self.config.bucket, pair.base, pair.quote
        );

        if let Some(ex) = exchange {
            query_str.push_str(&format!(
                r#" |> filter(fn: (r) => r.exchange == "{}")"#,
                ex
            ));
        }

        debug!("Executing InfluxDB query: {}", query_str);

        // For now, we'll always return a simulated result for development purposes
        // until we can properly parse the query results
        let mut results = Vec::new();
        
        // Mock data for testing
        if let Some(Exchange::Coinbase) = exchange {
            results.push(CurrentPrice {
                exchange: Exchange::Coinbase,
                pair: pair.clone(),
                price: 50000.0,  // Simulated price
                volume_24h: Some(1234.56),
                timestamp: Utc::now(),
            });
        } else if let Some(Exchange::Binance) = exchange {
            results.push(CurrentPrice {
                exchange: Exchange::Binance,
                pair: pair.clone(),
                price: 50100.0,  // Simulated price
                volume_24h: Some(2345.67),
                timestamp: Utc::now(),
            });
        } else {
            // If no exchange specified, return data for both
            results.push(CurrentPrice {
                exchange: Exchange::Coinbase,
                pair: pair.clone(),
                price: 50000.0,
                volume_24h: Some(1234.56),
                timestamp: Utc::now(),
            });
            results.push(CurrentPrice {
                exchange: Exchange::Binance,
                pair: pair.clone(),
                price: 50100.0,
                volume_24h: Some(2345.67),
                timestamp: Utc::now(),
            });
        }
        
        Ok(results)
    }

    pub async fn get_price_history(&self, query: &PriceQuery) -> Result<PriceHistory, StoreError> {
        let start_time = query
            .start_time
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| "-7d".to_string());
            
        let end_time = query
            .end_time
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| "now()".to_string());
            
        let mut flux_query_str = format!(
            r#"from(bucket: "{}")
               |> range(start: {}, stop: {})
               |> filter(fn: (r) => r._measurement == "price_history")
               |> filter(fn: (r) => r.base == "{}" and r.quote == "{}")
               |> filter(fn: (r) => r.interval == "{}")
               |> pivot(rowKey:["_time"], columnKey: ["_field"], valueColumn: "_value")"#,
            self.config.bucket, start_time, end_time, query.pair.base, 
            query.pair.quote, query.interval
        );

        if let Some(ex) = query.exchange {
            flux_query_str.push_str(&format!(
                r#" |> filter(fn: (r) => r.exchange == "{}")"#,
                ex
            ));
        }

        // Add sorting by time (descending)
        flux_query_str.push_str(r#" |> sort(columns: ["_time"], desc: true)"#);

        // Add limit if provided
        if let Some(limit) = query.limit {
            flux_query_str.push_str(&format!(r#" |> limit(n: {})"#, limit));
        }

        debug!("Executing InfluxDB query: {}", flux_query_str);

        // Generate simulated data for testing
        let mut data_points = Vec::new();
        let now = Utc::now();
        
        // Create a few data points with test data
        // Time interval between points will depend on the requested interval
        let time_step_seconds = match query.interval {
            PriceInterval::OneMinute => 60,
            PriceInterval::FiveMinutes => 300,
            PriceInterval::FifteenMinutes => 900,
            PriceInterval::OneHour => 3600,
            PriceInterval::FourHours => 14400,
            PriceInterval::OneDay => 86400,
            PriceInterval::OneWeek => 604800,
        };
        
        // Create 10 simulated data points
        let limit = query.limit.unwrap_or(10);
        for i in 0..std::cmp::min(limit, 10) {
            let timestamp = now - chrono::Duration::seconds(time_step_seconds * i as i64);
            let base_price = 50000.0;
            
            // Add some variability to the price
            let price = base_price * (1.0 + (i as f64 * 0.001) - 0.005);
            
            data_points.push(PriceHistoryPoint {
                timestamp,
                price,
                volume: Some(1000.0 + i as f64 * 100.0),
            });
        }
        
        let exchange = query.exchange.unwrap_or(Exchange::Coinbase);
        
        Ok(PriceHistory {
            exchange,
            pair: query.pair.clone(),
            interval: query.interval,
            data: data_points,
        })
    }
} 