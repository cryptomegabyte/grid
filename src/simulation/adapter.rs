// Integration adapter for simulation engine with live trading
// Converts between live trading types and simulation types

use crate::core::live_trading::SimulatedOrder as LiveOrder;
use crate::simulation::matching_engine::{SimulatedOrder, OrderSide, OrderType};
use crate::simulation::execution_simulator::ExecutionResult;
use crate::simulation::SimulationEngine;
use crate::clients::kraken_ws::parse_kraken_orderbook;
use serde_json::Value;

/// Adapter to integrate simulation engine with live trading system
pub struct SimulationAdapter {
    pub engine: SimulationEngine,
}

impl SimulationAdapter {
    /// Create new simulation adapter
    pub fn new() -> Self {
        Self {
            engine: SimulationEngine::kraken_simulator(),
        }
    }

    /// Initialize order book from Kraken WebSocket data
    pub fn update_from_kraken_ws(&mut self, pair: &str, ws_data: &Value) {
        // Try to parse order book update
        if let Some(kraken_book) = parse_kraken_orderbook(ws_data) {
            // Convert Kraken order book to snapshot
            let snapshot = SimulationEngine::kraken_to_snapshot(
                pair.to_string(),
                &kraken_book
            );
            
            // Initialize or update order book
            if self.engine.get_order_book(pair).is_none() {
                self.engine.initialize_order_book(pair.to_string(), snapshot);
            } else {
                // For updates, we re-initialize (could be optimized to use incremental updates)
                self.engine.initialize_order_book(pair.to_string(), snapshot);
            }
        }
    }

    /// Execute order using simulation engine
    pub fn execute_live_order(
        &mut self,
        live_order: &LiveOrder,
    ) -> Result<ExecutionResult, String> {
        // Convert live order to simulation order
        let sim_order = self.convert_to_sim_order(live_order)?;
        
        // Execute using simulation engine
        self.engine.execute_order(sim_order)
            .map_err(|e| e.to_string())
    }

    /// Convert live order to simulation order
    fn convert_to_sim_order(&self, live_order: &LiveOrder) -> Result<SimulatedOrder, String> {
        let side = match live_order.side.as_str() {
            "buy" => OrderSide::Buy,
            "sell" => OrderSide::Sell,
            _ => return Err(format!("Invalid order side: {}", live_order.side)),
        };

        // Assume limit orders by default
        let order_type = OrderType::Limit;
        
        Ok(SimulatedOrder {
            id: live_order.id.clone(),
            pair: live_order.pair.clone(),
            side,
            order_type,
            price: Some(live_order.price),
            quantity: live_order.quantity,
            timestamp: live_order.timestamp,
        })
    }

    /// Get simulation statistics
    pub fn get_stats(&self) -> String {
        let stats = self.engine.get_statistics();
        format!(
            "Orders: {} | Filled: {} | Partial: {} | Failed: {} | Volume: {:.2} | Fees: £{:.2} | Slippage: £{:.2} | Avg Latency: {:.0}ms",
            stats.total_orders,
            stats.successful_fills,
            stats.partial_fills,
            stats.failed_orders,
            stats.total_volume,
            stats.total_fees_paid,
            stats.total_slippage,
            stats.average_latency_ms
        )
    }

    /// Check if simulation engine is ready for a trading pair
    pub fn is_ready(&self, pair: &str) -> bool {
        self.engine.get_order_book(pair).is_some()
    }

    /// Get best prices from order book
    pub fn get_best_prices(&self, pair: &str) -> Option<(f64, f64)> {
        self.engine.get_best_prices(pair)
    }

    /// Get mid price
    pub fn get_mid_price(&self, pair: &str) -> Option<f64> {
        self.engine.get_mid_price(pair)
    }

    /// Get spread
    pub fn get_spread(&self, pair: &str) -> Option<f64> {
        self.engine.get_spread(pair)
    }

    /// Get order book health
    pub fn get_health(&self, pair: &str) -> Option<String> {
        let health = self.engine.get_order_book_health(pair)?;
        Some(format!(
            "{}: {} bids / {} asks | Spread: {:.2} ({:.1}bps) | Liquidity: {:.2} | {}",
            health.pair,
            health.bid_depth,
            health.ask_depth,
            health.spread,
            health.spread_bps,
            health.liquidity_score,
            if health.is_healthy { "✅ Healthy" } else { "⚠️ Degraded" }
        ))
    }
}

impl Default for SimulationAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = SimulationAdapter::new();
        assert!(adapter.is_ready("ETHGBP") == false);
    }
}
