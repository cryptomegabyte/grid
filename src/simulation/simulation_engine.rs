// Simulation Engine Orchestrator
// Coordinates order book, matching engine, and execution simulator

use crate::simulation::order_book::{LocalOrderBook, OrderBookSnapshot, OrderBookUpdate};
use crate::simulation::matching_engine::{
    OrderMatchingEngine, MatchingConfig, SimulatedOrder
};
use crate::simulation::execution_simulator::{
    ExecutionSimulator, ExecutionConfig, ExecutionResult, SlippageModel
};
use crate::clients::kraken_ws::OrderBook as KrakenOrderBook;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tracing::{info, warn, debug};

/// Main simulation engine that manages all trading pairs
pub struct SimulationEngine {
    /// Order books for each trading pair
    order_books: HashMap<String, LocalOrderBook>,
    /// Order matching engine
    matching_engine: OrderMatchingEngine,
    /// Execution simulator
    execution_simulator: ExecutionSimulator,
    /// Configuration
    config: SimulationConfig,
    /// Statistics
    stats: SimulationStats,
}

/// Configuration for simulation engine
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    pub matching_config: MatchingConfig,
    pub execution_config: ExecutionConfig,
    pub enable_logging: bool,
    pub track_statistics: bool,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            matching_config: MatchingConfig::default(),
            execution_config: ExecutionConfig::default(),
            enable_logging: true,
            track_statistics: true,
        }
    }
}

/// Statistics tracked by the simulation engine
#[derive(Debug, Clone, Default)]
pub struct SimulationStats {
    pub total_orders: u64,
    pub successful_fills: u64,
    pub partial_fills: u64,
    pub failed_orders: u64,
    pub total_volume: f64,
    pub total_fees_paid: f64,
    pub total_slippage: f64,
    pub average_latency_ms: f64,
}

impl SimulationEngine {
    /// Create a new simulation engine
    pub fn new(config: SimulationConfig) -> Self {
        Self {
            order_books: HashMap::new(),
            matching_engine: OrderMatchingEngine::new(config.matching_config.clone()),
            execution_simulator: ExecutionSimulator::new(config.execution_config.clone()),
            config,
            stats: SimulationStats::default(),
        }
    }

    /// Create simulation engine with default configuration
    pub fn with_default_config() -> Self {
        Self::new(SimulationConfig::default())
    }

    /// Create Kraken-specific simulation engine
    pub fn kraken_simulator() -> Self {
        Self::new(SimulationConfig {
            matching_config: MatchingConfig::default(),
            execution_config: ExecutionConfig {
                slippage_model: SlippageModel::Realistic {
                    base_spread_capture: 0.5,
                    volume_impact: 0.0005,
                    volatility_factor: 1.0,
                },
                ..Default::default()
            },
            enable_logging: true,
            track_statistics: true,
        })
    }

    /// Initialize order book for a trading pair
    pub fn initialize_order_book(&mut self, pair: String, snapshot: OrderBookSnapshot) {
        let order_book = LocalOrderBook::from_snapshot(snapshot);
        
        if self.config.enable_logging {
            info!("📖 Initialized order book for {}: {} bids, {} asks, spread: {:.4}",
                pair,
                order_book.bids.len(),
                order_book.asks.len(),
                order_book.spread().unwrap_or(0.0)
            );
        }

        self.order_books.insert(pair, order_book);
    }

    /// Update order book with incremental changes
    pub fn update_order_book(&mut self, pair: &str, update: OrderBookUpdate) {
        if let Some(order_book) = self.order_books.get_mut(pair) {
            order_book.apply_update(update);
            
            if self.config.enable_logging {
                debug!("📊 Updated order book for {}: BBO {:.2}/{:.2}",
                    pair,
                    order_book.best_bid().map(|b| b.price).unwrap_or(0.0),
                    order_book.best_ask().map(|a| a.price).unwrap_or(0.0)
                );
            }
        } else {
            warn!("⚠️  Attempted to update non-existent order book: {}", pair);
        }
    }

    /// Convert Kraken WebSocket order book to local snapshot
    pub fn kraken_to_snapshot(pair: String, kraken_book: &KrakenOrderBook) -> OrderBookSnapshot {
        let bids: Vec<(f64, f64)> = kraken_book.bids
            .iter()
            .map(|level| (level.price, level.volume))
            .collect();

        let asks: Vec<(f64, f64)> = kraken_book.asks
            .iter()
            .map(|level| (level.price, level.volume))
            .collect();

        OrderBookSnapshot {
            pair,
            bids,
            asks,
            timestamp: Utc::now(),
        }
    }

    /// Execute a simulated order
    pub fn execute_order(
        &mut self,
        order: SimulatedOrder,
    ) -> Result<ExecutionResult, SimulationError> {
        // Get order book for the pair
        let order_book = self.order_books.get(&order.pair)
            .ok_or_else(|| SimulationError::OrderBookNotFound(order.pair.clone()))?;

        // Validate order book state
        if let Err(e) = order_book.validate() {
            return Err(SimulationError::InvalidOrderBook(e));
        }

        // Match order against order book
        let match_result = self.matching_engine.match_order(order.clone(), order_book);

        // Calculate market parameters for execution simulation
        let spread = order_book.spread().unwrap_or(0.0);
        let liquidity = order_book.liquidity_score(10);

        // Simulate execution
        let execution_result = self.execution_simulator.simulate_execution(
            match_result,
            order.quantity,
            spread,
            liquidity,
        );

        // Update statistics
        if self.config.track_statistics {
            self.update_statistics(&execution_result);
        }

        // Log execution
        if self.config.enable_logging {
            self.log_execution(&order, &execution_result);
        }

        Ok(execution_result)
    }

    /// Execute multiple orders (batch processing)
    pub fn execute_orders(
        &mut self,
        orders: Vec<SimulatedOrder>,
    ) -> Vec<Result<ExecutionResult, SimulationError>> {
        orders.into_iter()
            .map(|order| self.execute_order(order))
            .collect()
    }

    /// Get current order book state
    pub fn get_order_book(&self, pair: &str) -> Option<&LocalOrderBook> {
        self.order_books.get(pair)
    }

    /// Get best bid/ask for a pair
    pub fn get_best_prices(&self, pair: &str) -> Option<(f64, f64)> {
        let book = self.order_books.get(pair)?;
        let bid = book.best_bid()?.price;
        let ask = book.best_ask()?.price;
        Some((bid, ask))
    }

    /// Get mid price for a pair
    pub fn get_mid_price(&self, pair: &str) -> Option<f64> {
        self.order_books.get(pair)?.mid_price()
    }

    /// Get current spread for a pair
    pub fn get_spread(&self, pair: &str) -> Option<f64> {
        self.order_books.get(pair)?.spread()
    }

    /// Calculate market impact for a potential order
    pub fn calculate_market_impact(
        &self,
        order: &SimulatedOrder,
    ) -> Result<crate::simulation::matching_engine::MarketImpact, SimulationError> {
        let order_book = self.order_books.get(&order.pair)
            .ok_or_else(|| SimulationError::OrderBookNotFound(order.pair.clone()))?;

        Ok(self.matching_engine.calculate_market_impact(order, order_book))
    }

    /// Get simulation statistics
    pub fn get_statistics(&self) -> &SimulationStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_statistics(&mut self) {
        self.stats = SimulationStats::default();
    }

    /// Update internal statistics
    fn update_statistics(&mut self, result: &ExecutionResult) {
        use crate::simulation::execution_simulator::ExecutionStatus;
        
        self.stats.total_orders += 1;
        
        match result.status {
            ExecutionStatus::Success => self.stats.successful_fills += 1,
            ExecutionStatus::PartialFill => self.stats.partial_fills += 1,
            ExecutionStatus::Failed | ExecutionStatus::Timeout => self.stats.failed_orders += 1,
        }

        self.stats.total_volume += result.total_filled;
        self.stats.total_fees_paid += result.total_fees;
        self.stats.total_slippage += result.total_slippage;

        // Update average latency (running average)
        let n = self.stats.total_orders as f64;
        self.stats.average_latency_ms = 
            (self.stats.average_latency_ms * (n - 1.0) + result.execution_time_ms as f64) / n;
    }

    /// Log execution details
    fn log_execution(&self, order: &SimulatedOrder, result: &ExecutionResult) {
        use crate::simulation::execution_simulator::ExecutionStatus;
        
        match result.status {
            ExecutionStatus::Success => {
                info!("✅ ORDER FILLED: {} {:?} {} @ £{:.4} | Qty: {:.4} | Fee: £{:.2} | Slippage: £{:.2} | Latency: {}ms",
                    order.pair,
                    order.side,
                    order.quantity,
                    result.average_price,
                    result.total_filled,
                    result.total_fees,
                    result.total_slippage,
                    result.execution_time_ms
                );
            }
            ExecutionStatus::PartialFill => {
                warn!("⚠️  PARTIAL FILL: {} {:?} {}/{} @ £{:.4}",
                    order.pair,
                    order.side,
                    result.total_filled,
                    order.quantity,
                    result.average_price
                );
            }
            ExecutionStatus::Failed | ExecutionStatus::Timeout => {
                warn!("❌ ORDER FAILED: {} {:?} {}",
                    order.pair,
                    order.side,
                    order.quantity
                );
            }
        }
    }

    /// Get list of all trading pairs with order books
    pub fn get_active_pairs(&self) -> Vec<String> {
        self.order_books.keys().cloned().collect()
    }

    /// Remove order book for a pair
    pub fn remove_order_book(&mut self, pair: &str) -> Option<LocalOrderBook> {
        self.order_books.remove(pair)
    }

    /// Clear all order books
    pub fn clear_all_order_books(&mut self) {
        self.order_books.clear();
    }

    /// Get order book health status
    pub fn get_order_book_health(&self, pair: &str) -> Option<OrderBookHealth> {
        let book = self.order_books.get(pair)?;
        
        let (bid_depth, ask_depth) = book.depth();
        let spread = book.spread()?;
        let mid_price = book.mid_price()?;
        let liquidity = book.liquidity_score(10);
        
        let spread_bps = (spread / mid_price) * 10000.0;
        
        Some(OrderBookHealth {
            pair: pair.to_string(),
            bid_depth,
            ask_depth,
            spread,
            spread_bps,
            mid_price,
            liquidity_score: liquidity,
            last_update: book.last_update,
            is_healthy: bid_depth > 5 && ask_depth > 5 && spread_bps < 100.0,
        })
    }
}

/// Order book health information
#[derive(Debug, Clone)]
pub struct OrderBookHealth {
    pub pair: String,
    pub bid_depth: usize,
    pub ask_depth: usize,
    pub spread: f64,
    pub spread_bps: f64,
    pub mid_price: f64,
    pub liquidity_score: f64,
    pub last_update: DateTime<Utc>,
    pub is_healthy: bool,
}

/// Simulation errors
#[derive(Debug, Clone)]
pub enum SimulationError {
    OrderBookNotFound(String),
    InvalidOrderBook(String),
    InsufficientLiquidity,
    InvalidOrder(String),
}

impl std::fmt::Display for SimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimulationError::OrderBookNotFound(pair) => 
                write!(f, "Order book not found for pair: {}", pair),
            SimulationError::InvalidOrderBook(reason) => 
                write!(f, "Invalid order book: {}", reason),
            SimulationError::InsufficientLiquidity => 
                write!(f, "Insufficient liquidity to execute order"),
            SimulationError::InvalidOrder(reason) => 
                write!(f, "Invalid order: {}", reason),
        }
    }
}

impl std::error::Error for SimulationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::order_book::OrderBookSnapshot;
    use crate::simulation::matching_engine::{SimulatedOrder, OrderSide, OrderType};

    fn create_test_snapshot(pair: &str) -> OrderBookSnapshot {
        OrderBookSnapshot {
            pair: pair.to_string(),
            bids: vec![
                (2000.0, 1.0),
                (1999.0, 2.0),
                (1998.0, 3.0),
            ],
            asks: vec![
                (2001.0, 1.0),
                (2002.0, 2.0),
                (2003.0, 3.0),
            ],
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_simulation_engine_creation() {
        let engine = SimulationEngine::with_default_config();
        assert_eq!(engine.get_active_pairs().len(), 0);
    }

    #[test]
    fn test_order_book_initialization() {
        let mut engine = SimulationEngine::with_default_config();
        let snapshot = create_test_snapshot("ETHGBP");
        
        engine.initialize_order_book("ETHGBP".to_string(), snapshot);
        assert_eq!(engine.get_active_pairs().len(), 1);
        assert!(engine.get_order_book("ETHGBP").is_some());
    }

    #[test]
    fn test_order_execution() {
        let mut engine = SimulationEngine::with_default_config();
        let snapshot = create_test_snapshot("ETHGBP");
        engine.initialize_order_book("ETHGBP".to_string(), snapshot);

        let order = SimulatedOrder {
            id: "test-1".to_string(),
            pair: "ETHGBP".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: None,
            quantity: 1.0,
            timestamp: Utc::now(),
        };

        let result = engine.execute_order(order);
        assert!(result.is_ok());
        
        let stats = engine.get_statistics();
        assert_eq!(stats.total_orders, 1);
    }
}
