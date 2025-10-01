// Advanced transaction cost modeling with realistic market microstructure

use crate::backtesting::{TradeType, TradingCosts, SlippageModel};
use chrono::{DateTime, Utc};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct TransactionCostModel {
    trading_costs: TradingCosts,
    slippage_model: SlippageModel,
    market_microstructure: MarketMicrostructure,
    execution_engine: ExecutionEngine,
}

#[derive(Debug, Clone)]
pub struct MarketMicrostructure {
    pub current_spread: f64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_size: f64,
    pub ask_size: f64,
    pub recent_volumes: VecDeque<f64>,
    pub volatility: f64,
    pub liquidity_state: LiquidityState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiquidityState {
    High,      // Deep order book, tight spreads
    Normal,    // Standard market conditions
    Low,       // Shallow order book, wide spreads
    Stressed,  // Very poor liquidity, potential for gaps
}

#[derive(Debug, Clone)]
pub struct ExecutionEngine {
    pub execution_style: ExecutionStyle,
    pub urgency_factor: f64,      // 0.0 = patient, 1.0 = aggressive
    pub slice_large_orders: bool,  // Break large orders into smaller pieces
    pub max_order_slice: f64,     // Maximum size per slice
}

#[derive(Debug, Clone)]
pub enum ExecutionStyle {
    MarketOrder,                  // Immediate execution, high cost
    LimitOrder { offset_bps: f64 }, // Better price, execution risk
    TWAP { duration_minutes: u32 }, // Time-weighted average price
    Iceberg { visible_size: f64 },  // Hide order size
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub intended_quantity: f64,
    pub executed_quantity: f64,
    pub average_price: f64,
    pub total_fees: f64,
    pub total_slippage: f64,
    pub market_impact: f64,
    pub execution_time_ms: u64,
    pub fill_ratio: f64,
    pub price_improvement: f64,    // Positive if we got better than expected
}

impl TransactionCostModel {
    pub fn new(trading_costs: TradingCosts, slippage_model: SlippageModel) -> Self {
        Self {
            trading_costs,
            slippage_model,
            market_microstructure: MarketMicrostructure::default(),
            execution_engine: ExecutionEngine::default(),
        }
    }

    /// Update market microstructure with new market data
    pub fn update_market_state(
        &mut self,
        mid_price: f64,
        volume: f64,
        volatility: f64,
        spread_bps: Option<f64>,
    ) {
        // Update spread
        let spread = spread_bps.unwrap_or(self.trading_costs.typical_spread_bps) / 10000.0 * mid_price;
        self.market_microstructure.current_spread = spread;
        self.market_microstructure.bid_price = mid_price - spread / 2.0;
        self.market_microstructure.ask_price = mid_price + spread / 2.0;
        
        // Update volume history
        self.market_microstructure.recent_volumes.push_back(volume);
        if self.market_microstructure.recent_volumes.len() > 20 {
            self.market_microstructure.recent_volumes.pop_front();
        }
        
        // Update volatility
        self.market_microstructure.volatility = volatility;
        
        // Determine liquidity state
        self.market_microstructure.liquidity_state = self.assess_liquidity_state(volume, spread_bps);
        
        // Estimate order book depth
        self.estimate_order_book_depth(volume, volatility);
    }

    fn assess_liquidity_state(&self, volume: f64, spread_bps: Option<f64>) -> LiquidityState {
        let avg_volume = if !self.market_microstructure.recent_volumes.is_empty() {
            self.market_microstructure.recent_volumes.iter().sum::<f64>() / 
            self.market_microstructure.recent_volumes.len() as f64
        } else {
            volume
        };
        
        let relative_volume = volume / avg_volume.max(1.0);
        let spread = spread_bps.unwrap_or(self.trading_costs.typical_spread_bps);
        
        match (relative_volume, spread) {
            (vol, spr) if vol > 2.0 && spr < 3.0 => LiquidityState::High,
            (vol, spr) if vol < 0.3 || spr > 15.0 => LiquidityState::Stressed,
            (vol, spr) if vol < 0.7 || spr > 8.0 => LiquidityState::Low,
            _ => LiquidityState::Normal,
        }
    }

    fn estimate_order_book_depth(&mut self, volume: f64, volatility: f64) {
        // Estimate typical order book depth based on volume and volatility
        let base_depth = volume * 0.1; // Assume 10% of volume is typical depth
        
        // Adjust for volatility (higher vol = shallower book)
        let vol_adjustment = 1.0 / (1.0 + volatility * 5.0);
        
        // Adjust for liquidity state
        let liquidity_adjustment = match self.market_microstructure.liquidity_state {
            LiquidityState::High => 1.5,
            LiquidityState::Normal => 1.0,
            LiquidityState::Low => 0.6,
            LiquidityState::Stressed => 0.3,
        };
        
        let estimated_depth = base_depth * vol_adjustment * liquidity_adjustment;
        
        self.market_microstructure.bid_size = estimated_depth;
        self.market_microstructure.ask_size = estimated_depth;
    }

    /// Execute a trade with realistic cost modeling
    pub fn execute_trade(
        &self,
        trade_type: TradeType,
        quantity: f64,
        timestamp: DateTime<Utc>,
    ) -> ExecutionResult {
        match &self.execution_engine.execution_style {
            ExecutionStyle::MarketOrder => {
                self.execute_market_order(trade_type, quantity, timestamp)
            }
            ExecutionStyle::LimitOrder { offset_bps } => {
                self.execute_limit_order(trade_type, quantity, *offset_bps, timestamp)
            }
            ExecutionStyle::TWAP { duration_minutes } => {
                self.execute_twap_order(trade_type, quantity, *duration_minutes, timestamp)
            }
            ExecutionStyle::Iceberg { visible_size } => {
                self.execute_iceberg_order(trade_type, quantity, *visible_size, timestamp)
            }
        }
    }

    fn execute_market_order(
        &self,
        trade_type: TradeType,
        quantity: f64,
        _timestamp: DateTime<Utc>,
    ) -> ExecutionResult {
        let (reference_price, _crossing_spread) = match trade_type {
            TradeType::Buy => (self.market_microstructure.ask_price, true),
            TradeType::Sell => (self.market_microstructure.bid_price, true),
        };

        // Calculate slippage components
        let base_slippage = self.calculate_base_slippage(quantity);
        let market_impact = self.calculate_market_impact(quantity);
        let volatility_slippage = self.calculate_volatility_slippage();
        let liquidity_penalty = self.calculate_liquidity_penalty(quantity);

        let total_slippage = base_slippage + market_impact + volatility_slippage + liquidity_penalty;
        
        // Apply slippage direction
        let execution_price = match trade_type {
            TradeType::Buy => reference_price + total_slippage,
            TradeType::Sell => reference_price - total_slippage,
        };

        // Calculate fees
        let fee_rate = self.trading_costs.taker_fee_rate; // Market orders are taker
        let total_fees = execution_price * quantity * fee_rate;

        // Market orders typically fill completely but with high cost
        let fill_ratio = self.calculate_fill_probability(quantity, true);
        let executed_quantity = quantity * fill_ratio;

        ExecutionResult {
            intended_quantity: quantity,
            executed_quantity,
            average_price: execution_price,
            total_fees,
            total_slippage,
            market_impact,
            execution_time_ms: 50, // Fast execution
            fill_ratio,
            price_improvement: 0.0, // No improvement for market orders
        }
    }

    fn execute_limit_order(
        &self,
        trade_type: TradeType,
        quantity: f64,
        offset_bps: f64,
        _timestamp: DateTime<Utc>,
    ) -> ExecutionResult {
        let mid_price = (self.market_microstructure.bid_price + self.market_microstructure.ask_price) / 2.0;
        let offset = mid_price * offset_bps / 10000.0;
        
        let limit_price = match trade_type {
            TradeType::Buy => mid_price - offset,  // Buy below mid
            TradeType::Sell => mid_price + offset, // Sell above mid
        };

        // Calculate fill probability based on how aggressive the limit price is
        let aggressiveness = match trade_type {
            TradeType::Buy => (self.market_microstructure.ask_price - limit_price) / self.market_microstructure.current_spread,
            TradeType::Sell => (limit_price - self.market_microstructure.bid_price) / self.market_microstructure.current_spread,
        };

        let fill_probability = self.calculate_limit_order_fill_probability(aggressiveness);
        let executed_quantity = quantity * fill_probability;

        // Limit orders get maker rebates or pay taker fees depending on execution
        let fee_rate = if aggressiveness > 0.5 {
            self.trading_costs.taker_fee_rate
        } else {
            -self.trading_costs.maker_fee_rate // Negative = rebate
        };

        let total_fees = limit_price * executed_quantity * fee_rate.abs();
        
        // Limited slippage for limit orders (mainly from partial fills)
        let partial_fill_slippage = if fill_probability < 1.0 {
            mid_price * 0.0001 * (1.0 - fill_probability) // Small penalty for partial fills
        } else {
            0.0
        };

        ExecutionResult {
            intended_quantity: quantity,
            executed_quantity,
            average_price: limit_price,
            total_fees,
            total_slippage: partial_fill_slippage,
            market_impact: 0.0, // Minimal market impact for limit orders
            execution_time_ms: if fill_probability > 0.9 { 200 } else { 5000 }, // Variable timing
            fill_ratio: fill_probability,
            price_improvement: offset.abs(), // Improvement vs market order
        }
    }

    fn execute_twap_order(
        &self,
        trade_type: TradeType,
        quantity: f64,
        duration_minutes: u32,
        _timestamp: DateTime<Utc>,
    ) -> ExecutionResult {
        // TWAP spreads order over time to reduce market impact
        let num_slices = (duration_minutes / 5).max(1); // Every 5 minutes
        let slice_size = quantity / num_slices as f64;
        
        // Reduced market impact due to time distribution
        let impact_reduction = 0.7; // 30% impact reduction
        let adjusted_impact = self.calculate_market_impact(slice_size) * impact_reduction;
        
        let mid_price = (self.market_microstructure.bid_price + self.market_microstructure.ask_price) / 2.0;
        let execution_price = match trade_type {
            TradeType::Buy => mid_price + adjusted_impact,
            TradeType::Sell => mid_price - adjusted_impact,
        };

        // Mix of maker and taker fees
        let avg_fee_rate = (self.trading_costs.maker_fee_rate + self.trading_costs.taker_fee_rate) / 2.0;
        let total_fees = execution_price * quantity * avg_fee_rate;

        ExecutionResult {
            intended_quantity: quantity,
            executed_quantity: quantity * 0.95, // 95% fill rate for TWAP
            average_price: execution_price,
            total_fees,
            total_slippage: adjusted_impact,
            market_impact: adjusted_impact,
            execution_time_ms: duration_minutes as u64 * 60 * 1000, // Full duration
            fill_ratio: 0.95,
            price_improvement: self.calculate_market_impact(quantity) - adjusted_impact,
        }
    }

    fn execute_iceberg_order(
        &self,
        trade_type: TradeType,
        quantity: f64,
        visible_size: f64,
        _timestamp: DateTime<Utc>,
    ) -> ExecutionResult {
        // Iceberg orders hide size but may take longer to execute
        let num_icebergs = (quantity / visible_size).ceil() as u32;
        
        // Reduced market impact due to hidden size
        let impact_reduction = 0.8;
        let adjusted_impact = self.calculate_market_impact(visible_size) * impact_reduction;
        
        let mid_price = (self.market_microstructure.bid_price + self.market_microstructure.ask_price) / 2.0;
        let execution_price = match trade_type {
            TradeType::Buy => mid_price + adjusted_impact,
            TradeType::Sell => mid_price - adjusted_impact,
        };

        // Primarily maker fees due to patient execution
        let total_fees = execution_price * quantity * self.trading_costs.maker_fee_rate;

        ExecutionResult {
            intended_quantity: quantity,
            executed_quantity: quantity * 0.92, // 92% fill rate due to complexity
            average_price: execution_price,
            total_fees,
            total_slippage: adjusted_impact,
            market_impact: adjusted_impact * num_icebergs as f64 * 0.3, // Cumulative but reduced impact
            execution_time_ms: num_icebergs as u64 * 1000, // 1 second per iceberg
            fill_ratio: 0.92,
            price_improvement: self.calculate_market_impact(quantity) - adjusted_impact,
        }
    }

    // Slippage calculation methods
    fn calculate_base_slippage(&self, quantity: f64) -> f64 {
        let mid_price = (self.market_microstructure.bid_price + self.market_microstructure.ask_price) / 2.0;
        mid_price * self.slippage_model.base_slippage_bps / 10000.0 * 
        (1.0 + quantity / 1000.0) // Increase with size
    }

    fn calculate_market_impact(&self, quantity: f64) -> f64 {
        let notional_value = quantity * 
            (self.market_microstructure.bid_price + self.market_microstructure.ask_price) / 2.0;
        
        // Square root law of market impact
        notional_value.sqrt() * self.slippage_model.market_impact_factor *
        match self.market_microstructure.liquidity_state {
            LiquidityState::High => 0.7,
            LiquidityState::Normal => 1.0,
            LiquidityState::Low => 1.5,
            LiquidityState::Stressed => 2.5,
        }
    }

    fn calculate_volatility_slippage(&self) -> f64 {
        let mid_price = (self.market_microstructure.bid_price + self.market_microstructure.ask_price) / 2.0;
        mid_price * self.market_microstructure.volatility * self.slippage_model.volatility_multiplier
    }

    fn calculate_liquidity_penalty(&self, quantity: f64) -> f64 {
        let available_liquidity = match self.market_microstructure.liquidity_state {
            LiquidityState::High => self.market_microstructure.bid_size * 2.0,
            LiquidityState::Normal => self.market_microstructure.bid_size,
            LiquidityState::Low => self.market_microstructure.bid_size * 0.5,
            LiquidityState::Stressed => self.market_microstructure.bid_size * 0.2,
        };

        if quantity > available_liquidity {
            let excess_ratio = (quantity - available_liquidity) / available_liquidity;
            let mid_price = (self.market_microstructure.bid_price + self.market_microstructure.ask_price) / 2.0;
            mid_price * excess_ratio * 0.01 // 1% penalty per excess ratio
        } else {
            0.0
        }
    }

    fn calculate_fill_probability(&self, quantity: f64, is_market_order: bool) -> f64 {
        if is_market_order {
            let available_liquidity = self.market_microstructure.bid_size;
            if quantity <= available_liquidity {
                1.0
            } else {
                // Partial fill probability decreases with size
                (available_liquidity / quantity).min(1.0)
            }
        } else {
            0.85 // Default limit order fill probability
        }
    }

    fn calculate_limit_order_fill_probability(&self, aggressiveness: f64) -> f64 {
        // More aggressive orders fill faster but at worse prices
        let base_probability = match self.market_microstructure.liquidity_state {
            LiquidityState::High => 0.9,
            LiquidityState::Normal => 0.8,
            LiquidityState::Low => 0.6,
            LiquidityState::Stressed => 0.4,
        };

        // Adjust based on how aggressive the order is
        if aggressiveness > 1.0 {
            base_probability // Very aggressive, likely to fill
        } else if aggressiveness > 0.5 {
            base_probability * 0.8
        } else if aggressiveness > 0.0 {
            base_probability * 0.6
        } else {
            base_probability * 0.3 // Passive orders fill less frequently
        }
    }
}

impl Default for MarketMicrostructure {
    fn default() -> Self {
        Self {
            current_spread: 0.001, // 0.1% default spread
            bid_price: 0.999,
            ask_price: 1.001,
            bid_size: 1000.0,
            ask_size: 1000.0,
            recent_volumes: VecDeque::new(),
            volatility: 0.02, // 2% volatility
            liquidity_state: LiquidityState::Normal,
        }
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self {
            execution_style: ExecutionStyle::LimitOrder { offset_bps: 5.0 },
            urgency_factor: 0.3, // Moderately patient
            slice_large_orders: true,
            max_order_slice: 500.0, // £500 max per slice
        }
    }
}