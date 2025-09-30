// Grid Trading Bot Library
// 
// A modular cryptocurrency grid trading bot with vectorized backtesting and Markov chain analysis

pub mod core;
pub mod clients;
pub mod config;
pub mod backtesting;

// Re-export core trading types
pub use core::{MarketState, GridSignal, GridTrader, MarketAnalyzer};

// Re-export client types
pub use clients::{KrakenWebSocketClient, KrakenHistoricalClient};

// Re-export configuration
pub use config::{Config, TradingConfig, MarketConfig, LoggingConfig, ConfigError};

// Re-export backtesting components
pub use backtesting::{
    BacktestConfig, BacktestResult, HistoricalData, Trade, TradeType, PerformanceMetrics,
    engine::{BacktestingEngine, BacktestBuilder, BacktestError},
    vectorized::{VectorizedGridProcessor, ParameterGrid, StrategyResult},
    analytics::PerformanceAnalyzer,
    markov::{MarkovChainAnalyzer, MarketStatePrediction},
};