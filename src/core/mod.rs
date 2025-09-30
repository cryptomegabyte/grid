// Core trading logic modules

pub mod types;
pub mod grid_trader;
pub mod market_state;

// Re-export commonly used types
pub use types::{MarketState, GridSignal};
pub use grid_trader::GridTrader;
pub use market_state::MarketAnalyzer;