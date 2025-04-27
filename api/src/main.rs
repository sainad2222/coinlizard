mod config;
mod handler;
mod service;

use axum::{
    routing::{get, post},
    Router,
};
use common::models::{TradingPair, Exchange, PriceInterval};
use connectors::{binance::BinanceConnector, coinbase::CoinbaseConnector, ExchangeConnector};
use service::CoinService;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{debug, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting CoinLizard API");

    // Load configuration from environment
    let store_config = store::StoreConfig::from_env()
        .map_err(|e| format!("Failed to load store configuration: {}", e))?;

    // Create InfluxDB store
    let price_store = store::PriceStore::new(store_config)
        .map_err(|e| format!("Failed to create price store: {}", e))?;

    // Create exchange connectors
    let coinbase = Arc::new(CoinbaseConnector::new());
    let binance = Arc::new(BinanceConnector::new());

    // Create coin service
    let service = Arc::new(RwLock::new(CoinService::new(
        coinbase,
        binance,
        Arc::new(price_store),
    )));

    // Create CORS middleware
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);

    // Create Axum router with API routes
    let app = Router::new()
        .route("/api/v1/coins", get(handler::list_coins))
        .route(
            "/api/v1/coins/:id/price",
            get(handler::get_current_price),
        )
        .route(
            "/api/v1/coins/:id/history/daily",
            get(handler::get_price_history),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(service);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
} 