// Grid Trading Bot Library
// 
// A modular cryptocurrency grid trading bot with vectorized backtesting and Markov chain analysis

pub mod core;
pub mod clients;
pub mod config;
pub mod backtesting;
pub mod optimization;

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

// Re-export optimization components
pub use optimization::{
    OptimizationConfig, ParameterSet, OptimizationResult, ParameterOptimizer,
    parameter_search::{ParameterSearchEngine, SearchStrategy},
    grid_optimizer::{GridOptimizer, GridStrategy},
    risk_optimizer::{RiskOptimizer, RiskModel, RiskMetrics},
};