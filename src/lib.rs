// Grid Trading Bot Library
// 
// A modular cryptocurrency grid trading bot with vectorized backtesting and Markov chain analysis

pub mod core;
pub mod clients;
pub mod config;
pub mod cli_config;  // New CLI-specific configuration
pub mod db;          // SQLite database layer
pub mod error;       // Unified error handling
pub mod validation;  // Pre-flight validation
pub mod backtesting;
pub mod optimization;

// Re-export core trading types
pub use core::{MarketState, GridSignal, GridTrader, MarketAnalyzer};

// Re-export error types
pub use error::{TradingError, TradingResult};

// Re-export validation types
pub use validation::{PreFlightValidator, ValidationResult, ValidationCheck, ValidationLevel};

// Re-export client types
pub use clients::{KrakenWebSocketClient, KrakenHistoricalClient};

// Re-export configuration
pub use config::{Config, TradingConfig, MarketConfig, LoggingConfig, ConfigError};

// Re-export CLI configuration
pub use cli_config::{CliConfig, CliConfigError, ApiConfig, TradingDefaults, OptimizationConfig as CliOptimizationConfig};

// Re-export database types
pub use db::{Database, Strategy, Trade as DbTrade, ExecutionHistory, StrategyService};

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