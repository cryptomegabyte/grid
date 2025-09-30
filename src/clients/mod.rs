// External API clients

pub mod kraken_ws;
pub mod kraken_api;

// Re-export client types
pub use kraken_ws::{KrakenWebSocketClient, parse_kraken_ticker, handle_kraken_event};
pub use kraken_api::{
    KrakenHistoricalClient, KrakenApiError, TradingPair,
    get_available_pairs, get_gbp_pairs, get_gbp_pair_names
};