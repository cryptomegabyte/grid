// Core trading logic modules

pub mod types;
pub mod grid_trader;
pub mod market_state;
pub mod live_trading;
pub mod error_handling;
pub mod position_manager;
pub mod monitoring;

// Re-export commonly used types
pub use types::{MarketState, GridSignal};
pub use grid_trader::GridTrader;
pub use market_state::MarketAnalyzer;
pub use live_trading::{LiveTradingEngine, OptimizedStrategy, GridMode};
pub use error_handling::{TradingError, CircuitBreaker, RetryPolicy, HealthMonitor, GracefulShutdown};
pub use position_manager::{PositionManager, Position, RiskLimits, PositionSizingMethod, TradeExecution, PortfolioSummary};
pub use monitoring::{TradingMonitor, SafetyLimits, PerformanceTracker, RealTimeMetrics, Alert, AlertLevel};