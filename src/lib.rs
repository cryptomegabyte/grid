// Grid Trading Bot Library
// 
// A modular cryptocurrency grid trading bot with Markov chain enhancements

pub mod types;
pub mod market_state;
pub mod grid_trader;
pub mod websocket_client;
pub mod config;

// Re-export commonly used types for convenience
pub use types::{MarketState, GridSignal};
pub use market_state::MarketAnalyzer;
pub use grid_trader::GridTrader;
pub use websocket_client::{KrakenWebSocketClient, parse_kraken_ticker, handle_kraken_event};
pub use config::{Config, TradingConfig, MarketConfig, LoggingConfig, ConfigError};