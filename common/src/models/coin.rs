use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Coin {
    /// Unique identifier for the coin (e.g., "bitcoin", "ethereum")
    pub id: String,
    /// Human-readable name (e.g., "Bitcoin", "Ethereum")
    pub name: String,
    /// Ticker symbol (e.g., "BTC", "ETH")
    pub symbol: String,
}

/// Exchange identifiers
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Exchange {
    #[serde(rename = "coinbase")]
    Coinbase,
    #[serde(rename = "binance")]
    Binance,
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exchange::Coinbase => write!(f, "coinbase"),
            Exchange::Binance => write!(f, "binance"),
        }
    }
}

/// Represents a pair of coins being traded
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TradingPair {
    pub base: String,   // Base currency (e.g., BTC)
    pub quote: String,  // Quote currency (e.g., USD)
} 