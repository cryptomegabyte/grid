// Grid Trading Bot Library
// 
// A modular cryptocurrency grid trading bot with Markov chain enhancements

pub mod types;
pub mod market_state;
pub mod grid_trader;
pub mod websocket_client;
pub mod config;
pub mod backtesting;

// Re-export commonly used types for convenience
pub use types::{MarketState, GridSignal};
pub use market_state::MarketAnalyzer;
pub use grid_trader::GridTrader;
pub use websocket_client::{KrakenWebSocketClient, parse_kraken_ticker, handle_kraken_event};
pub use config::{Config, TradingConfig, MarketConfig, LoggingConfig, ConfigError};

// Re-export backtesting components
pub use backtesting::{
    BacktestConfig, BacktestResult, HistoricalData, Trade, TradeType, PerformanceMetrics,
    engine::{BacktestingEngine, BacktestBuilder, BacktestError},
    kraken_api::{KrakenHistoricalClient, KrakenApiError},
    vectorized::{VectorizedGridProcessor, ParameterGrid, StrategyResult},
    analytics::PerformanceAnalyzer,
    markov::{MarkovChainAnalyzer, MarketStatePrediction},
};