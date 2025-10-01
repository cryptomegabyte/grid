// Execution Simulator
// Simulates realistic trade execution with latency, slippage, and market dynamics

use crate::simulation::matching_engine::{MatchResult, FillInfo, OrderStatus};
use chrono::{DateTime, Utc, Duration};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

/// Result of simulated execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub order_id: String,
    pub fills: Vec<ExecutedFill>,
    pub total_filled: f64,
    pub average_price: f64,
    pub total_fees: f64,
    pub total_slippage: f64,
    pub execution_time_ms: u64,
    pub status: ExecutionStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedFill {
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub slippage: f64,
    pub timestamp: DateTime<Utc>,
    pub latency_ms: u64,
    pub is_maker: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    PartialFill,
    Failed,
    Timeout,
}

/// Slippage model configuration
#[derive(Debug, Clone)]
pub enum SlippageModel {
    /// Fixed percentage slippage
    Fixed(f64),
    /// Square root model: slippage ∝ √(order_size/liquidity)
    SquareRoot { base_impact: f64 },
    /// Linear model: slippage ∝ (order_size/liquidity)
    Linear { impact_coefficient: f64 },
    /// Realistic model combining multiple factors
    Realistic {
        base_spread_capture: f64, // Percentage of spread captured
        volume_impact: f64,       // Impact per unit volume
        volatility_factor: f64,   // Volatility multiplier
    },
}

impl Default for SlippageModel {
    fn default() -> Self {
        SlippageModel::Realistic {
            base_spread_capture: 0.5, // Capture 50% of spread
            volume_impact: 0.001,      // 0.1% per unit volume
            volatility_factor: 1.0,    // No adjustment
        }
    }
}

/// Execution simulator configuration
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub slippage_model: SlippageModel,
    pub fee_config: FeeConfig,
    pub latency_config: LatencyConfig,
    pub fill_probability_config: FillProbabilityConfig,
}

/// Fee structure (Kraken-like)
#[derive(Debug, Clone)]
pub struct FeeConfig {
    pub maker_fee_bps: f64,  // Basis points (e.g., 16 = 0.16%)
    pub taker_fee_bps: f64,  // Basis points (e.g., 26 = 0.26%)
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            maker_fee_bps: 16.0,  // Kraken maker fee: 0.16%
            taker_fee_bps: 26.0,  // Kraken taker fee: 0.26%
        }
    }
}

/// Latency configuration
#[derive(Debug, Clone)]
pub struct LatencyConfig {
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub network_jitter_ms: u64,
    pub exchange_processing_ms: u64,
}

impl Default for LatencyConfig {
    fn default() -> Self {
        Self {
            min_latency_ms: 50,
            max_latency_ms: 200,
            network_jitter_ms: 20,
            exchange_processing_ms: 10,
        }
    }
}

/// Fill probability configuration
#[derive(Debug, Clone)]
pub struct FillProbabilityConfig {
    pub base_fill_rate: f64,        // Base probability of fill
    pub liquidity_threshold: f64,   // Liquidity factor threshold
    pub adverse_selection_factor: f64, // Probability of being front-run
}

impl Default for FillProbabilityConfig {
    fn default() -> Self {
        Self {
            base_fill_rate: 0.95,           // 95% base fill rate
            liquidity_threshold: 0.1,       // 10% of order book depth
            adverse_selection_factor: 0.05, // 5% chance of adverse selection
        }
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            slippage_model: SlippageModel::default(),
            fee_config: FeeConfig::default(),
            latency_config: LatencyConfig::default(),
            fill_probability_config: FillProbabilityConfig::default(),
        }
    }
}

/// Execution simulator
pub struct ExecutionSimulator {
    config: ExecutionConfig,
}

impl ExecutionSimulator {
    pub fn new(config: ExecutionConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(ExecutionConfig::default())
    }

    /// Simulate execution of a matched order
    pub fn simulate_execution(
        &self,
        match_result: MatchResult,
        order_quantity: f64,
        spread: f64,
        liquidity: f64,
    ) -> ExecutionResult {
        let start_time = Utc::now();

        // Check if order should be filled (simulate order rejection/timeout)
        if !self.should_fill(&match_result, order_quantity, liquidity) {
            return ExecutionResult {
                order_id: match_result.order_id,
                fills: Vec::new(),
                total_filled: 0.0,
                average_price: 0.0,
                total_fees: 0.0,
                total_slippage: 0.0,
                execution_time_ms: 0,
                status: ExecutionStatus::Failed,
                timestamp: start_time,
            };
        }

        // Simulate execution for each fill
        let mut executed_fills = Vec::new();
        let mut total_fees = 0.0;
        let mut total_slippage_cost = 0.0;

        for fill in &match_result.fills {
            let latency = self.simulate_latency();
            let slippage = self.calculate_slippage(fill, order_quantity, spread, liquidity);
            let actual_price = self.apply_slippage(fill.price, slippage);
            let fee = self.calculate_fee(fill, actual_price);

            executed_fills.push(ExecutedFill {
                price: actual_price,
                quantity: fill.quantity,
                fee,
                slippage: slippage * fill.quantity,
                timestamp: start_time + Duration::milliseconds(latency as i64),
                latency_ms: latency,
                is_maker: fill.is_maker,
            });

            total_fees += fee;
            total_slippage_cost += slippage * fill.quantity;
        }

        let execution_time = self.calculate_total_execution_time(&executed_fills);
        
        let status = match match_result.status {
            OrderStatus::FullyFilled => ExecutionStatus::Success,
            OrderStatus::PartiallyFilled => ExecutionStatus::PartialFill,
            OrderStatus::Rejected => ExecutionStatus::Failed,
            OrderStatus::PostedToBook => ExecutionStatus::Success, // Order posted, not executed
        };

        ExecutionResult {
            order_id: match_result.order_id,
            fills: executed_fills,
            total_filled: match_result.total_filled,
            average_price: match_result.average_price,
            total_fees,
            total_slippage: total_slippage_cost,
            execution_time_ms: execution_time,
            status,
            timestamp: start_time,
        }
    }

    /// Determine if order should be filled
    fn should_fill(
        &self,
        match_result: &MatchResult,
        order_quantity: f64,
        liquidity: f64,
    ) -> bool {
        if match_result.status == OrderStatus::Rejected {
            return false;
        }

        let mut rng = thread_rng();
        
        // Base fill probability
        let mut fill_prob = self.config.fill_probability_config.base_fill_rate;

        // Adjust for liquidity
        let liquidity_ratio = order_quantity / liquidity.max(0.0001);
        if liquidity_ratio > self.config.fill_probability_config.liquidity_threshold {
            fill_prob *= 0.8; // Reduce fill probability for large orders
        }

        // Simulate adverse selection (being front-run)
        let adverse_selection = rng.gen::<f64>() 
            < self.config.fill_probability_config.adverse_selection_factor;
        if adverse_selection {
            fill_prob *= 0.5; // 50% less likely to fill if front-run
        }

        rng.gen::<f64>() < fill_prob
    }

    /// Simulate network and exchange latency
    fn simulate_latency(&self) -> u64 {
        let mut rng = thread_rng();
        
        // Base latency
        let base = rng.gen_range(
            self.config.latency_config.min_latency_ms
                ..=self.config.latency_config.max_latency_ms
        );

        // Add jitter
        let jitter = rng.gen_range(0..=self.config.latency_config.network_jitter_ms);
        
        // Add exchange processing
        let processing = self.config.latency_config.exchange_processing_ms;

        base + jitter + processing
    }

    /// Calculate slippage for a fill
    fn calculate_slippage(
        &self,
        fill: &FillInfo,
        order_quantity: f64,
        spread: f64,
        liquidity: f64,
    ) -> f64 {
        match &self.config.slippage_model {
            SlippageModel::Fixed(pct) => fill.price * pct,
            
            SlippageModel::SquareRoot { base_impact } => {
                let impact_factor = (order_quantity / liquidity.max(0.0001)).sqrt();
                fill.price * base_impact * impact_factor
            }
            
            SlippageModel::Linear { impact_coefficient } => {
                let impact_factor = order_quantity / liquidity.max(0.0001);
                fill.price * impact_coefficient * impact_factor
            }
            
            SlippageModel::Realistic {
                base_spread_capture,
                volume_impact,
                volatility_factor,
            } => {
                // Capture portion of spread
                let spread_cost = spread * base_spread_capture;
                
                // Add volume-based impact
                let volume_ratio = fill.quantity / liquidity.max(0.0001);
                let volume_cost = fill.price * volume_impact * volume_ratio;
                
                // Apply volatility adjustment
                let total_slippage = (spread_cost + volume_cost) * volatility_factor;
                
                total_slippage
            }
        }
    }

    /// Apply slippage to price
    fn apply_slippage(&self, price: f64, slippage: f64) -> f64 {
        let mut rng = thread_rng();
        
        // Slippage is typically adverse (worse price)
        // But occasionally can be favorable (price improvement)
        let favorable_prob = 0.1; // 10% chance of price improvement
        
        if rng.gen::<f64>() < favorable_prob {
            (price - slippage).max(0.0) // Price improvement (buy lower, sell higher)
        } else {
            price + slippage // Adverse slippage
        }
    }

    /// Calculate trading fee
    fn calculate_fee(&self, fill: &FillInfo, execution_price: f64) -> f64 {
        let trade_value = execution_price * fill.quantity;
        let fee_bps = if fill.is_maker {
            self.config.fee_config.maker_fee_bps
        } else {
            self.config.fee_config.taker_fee_bps
        };
        
        trade_value * (fee_bps / 10000.0)
    }

    /// Calculate total execution time
    fn calculate_total_execution_time(&self, fills: &[ExecutedFill]) -> u64 {
        fills.iter()
            .map(|f| f.latency_ms)
            .max()
            .unwrap_or(0)
    }

    /// Create Kraken-like execution simulator
    pub fn kraken_simulator() -> Self {
        Self::new(ExecutionConfig {
            slippage_model: SlippageModel::Realistic {
                base_spread_capture: 0.5,
                volume_impact: 0.0005,
                volatility_factor: 1.0,
            },
            fee_config: FeeConfig {
                maker_fee_bps: 16.0, // 0.16%
                taker_fee_bps: 26.0, // 0.26%
            },
            latency_config: LatencyConfig {
                min_latency_ms: 50,
                max_latency_ms: 150,
                network_jitter_ms: 30,
                exchange_processing_ms: 20,
            },
            fill_probability_config: FillProbabilityConfig {
                base_fill_rate: 0.92,
                liquidity_threshold: 0.1,
                adverse_selection_factor: 0.08,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::matching_engine::FillInfo;

    #[test]
    fn test_execution_simulator_creation() {
        let _simulator = ExecutionSimulator::with_default_config();
        assert!(true); // Just test creation
    }

    #[test]
    fn test_latency_simulation() {
        let simulator = ExecutionSimulator::with_default_config();
        let latency = simulator.simulate_latency();
        assert!(latency >= 50 && latency <= 250); // Within reasonable bounds
    }

    #[test]
    fn test_fee_calculation() {
        let simulator = ExecutionSimulator::with_default_config();
        let fill = FillInfo {
            price: 2000.0,
            quantity: 1.0,
            timestamp: Utc::now(),
            is_maker: false,
        };
        
        let fee = simulator.calculate_fee(&fill, 2000.0);
        let expected = 2000.0 * 0.0026; // 0.26% taker fee
        assert!((fee - expected).abs() < 0.01);
    }
}
