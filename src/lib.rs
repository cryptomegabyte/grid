// Grid Trading Bot Library
// 
// A modular cryptocurrency grid trading bot with Markov chain enhancements

pub mod types;
pub mod market_state;
pub mod grid_trader;
pub mod websocket_client;

// Re-export commonly used types for convenience
pub use types::{MarketState, GridSignal, KRAKEN_WS_URL, TRADING_PAIR, GRID_LEVELS, GRID_SPACING, MIN_PRICE_CHANGE};
pub use market_state::MarketAnalyzer;
pub use grid_trader::GridTrader;
pub use websocket_client::{KrakenWebSocketClient, parse_kraken_ticker, handle_kraken_event};