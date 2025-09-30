// Backtesting data structures and types

pub mod engine;
pub mod vectorized;
pub mod analytics;
pub mod markov;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use ndarray::Array1;
use uuid::Uuid;
use crate::core::types::MarketState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OHLCData {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub struct HistoricalData {
    pub timestamps: Vec<DateTime<Utc>>,
    pub prices: Array1<f64>,           // Close prices for vectorized operations
    pub highs: Array1<f64>,
    pub lows: Array1<f64>,
    pub volumes: Array1<f64>,
    pub trading_pair: String,
    pub timeframe: String,
}

impl HistoricalData {
    pub fn from_ohlc(ohlc_data: Vec<OHLCData>, trading_pair: String, timeframe: String) -> Self {
        let len = ohlc_data.len();
        let mut timestamps = Vec::with_capacity(len);
        let mut prices = Vec::with_capacity(len);
        let mut highs = Vec::with_capacity(len);
        let mut lows = Vec::with_capacity(len);
        let mut volumes = Vec::with_capacity(len);

        for candle in ohlc_data {
            timestamps.push(candle.timestamp);
            prices.push(candle.close);
            highs.push(candle.high);
            lows.push(candle.low);
            volumes.push(candle.volume);
        }

        Self {
            timestamps,
            prices: Array1::from_vec(prices),
            highs: Array1::from_vec(highs),
            lows: Array1::from_vec(lows),
            volumes: Array1::from_vec(volumes),
            trading_pair,
            timeframe,
        }
    }

    pub fn len(&self) -> usize {
        self.prices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TradeType {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: Uuid,
    pub trade_type: TradeType,
    pub price: f64,
    pub quantity: f64,
    pub timestamp: DateTime<Utc>,
    pub grid_level: f64,
    
    // Cost analysis
    pub gross_pnl: f64,
    pub fees_paid: f64,
    pub slippage_cost: f64,
    pub net_pnl: f64,
    
    // Execution quality
    pub intended_price: f64,
    pub execution_delay_ms: u64,
    pub fill_probability: f64,
}

impl Trade {
    pub fn new(
        trade_type: TradeType,
        intended_price: f64,
        actual_price: f64,
        quantity: f64,
        timestamp: DateTime<Utc>,
        grid_level: f64,
        fees_paid: f64,
        slippage_cost: f64,
    ) -> Self {
        let gross_pnl = match trade_type {
            TradeType::Buy => 0.0, // PnL calculated when position is closed
            TradeType::Sell => (actual_price - intended_price) * quantity,
        };
        
        let net_pnl = gross_pnl - fees_paid - slippage_cost;

        Self {
            id: Uuid::new_v4(),
            trade_type,
            price: actual_price,
            quantity,
            timestamp,
            grid_level,
            gross_pnl,
            fees_paid,
            slippage_cost,
            net_pnl,
            intended_price,
            execution_delay_ms: 0,
            fill_probability: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TradingCosts {
    pub maker_fee_rate: f64,        // Kraken maker fee (e.g., 0.0016 for 0.16%)
    pub taker_fee_rate: f64,        // Kraken taker fee (e.g., 0.0026 for 0.26%)
    pub min_order_size: f64,        // Minimum order size
    pub typical_spread_bps: f64,    // Typical bid-ask spread in basis points
}

impl Default for TradingCosts {
    fn default() -> Self {
        Self {
            maker_fee_rate: 0.0016,     // 0.16% - Kraken Pro maker fee
            taker_fee_rate: 0.0026,     // 0.26% - Kraken Pro taker fee
            min_order_size: 0.50,       // £0.50 minimum (reduced from £1 for better grid trading)
            typical_spread_bps: 5.0,    // 5 basis points typical spread
        }
    }
}

#[derive(Debug, Clone)]
pub struct SlippageModel {
    pub base_slippage_bps: f64,         // Base slippage in basis points
    pub market_impact_factor: f64,       // Price impact per unit size
    pub volatility_multiplier: f64,      // Slippage increases with volatility
    pub liquidity_factor: f64,           // Lower liquidity = higher slippage
}

impl Default for SlippageModel {
    fn default() -> Self {
        Self {
            base_slippage_bps: 2.5,         // 2.5 bps base slippage
            market_impact_factor: 0.0001,   // 0.01% impact per £1000
            volatility_multiplier: 0.5,     // 50% volatility impact
            liquidity_factor: 1.0,          // Normal liquidity
        }
    }
}

#[derive(Debug, Clone)]
pub struct RiskConfig {
    pub max_position_size_pct: f64,     // Max position as % of capital
    pub max_daily_loss_pct: f64,        // Stop trading if daily loss exceeds
    pub max_drawdown_pct: f64,          // Emergency stop threshold
    pub min_time_between_trades_ms: u64, // Prevent over-trading
    pub volatility_threshold: f64,       // Don't trade if volatility too high
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_size_pct: 0.10,    // 10% of capital per position (increased from 2% for crypto trading)
            max_daily_loss_pct: 0.05,       // 5% daily loss limit
            max_drawdown_pct: 0.15,         // 15% drawdown stop
            min_time_between_trades_ms: 1000, // 1 second minimum
            volatility_threshold: 0.05,     // 5% volatility threshold
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    // Returns
    pub total_return_pct: f64,
    pub annualized_return_pct: f64,
    pub excess_return_pct: f64,         // Return above risk-free rate
    
    // Risk metrics
    pub volatility_pct: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown_pct: f64,
    pub value_at_risk_95: f64,          // 95% VaR
    pub conditional_var_95: f64,        // Expected shortfall
    
    // Trading statistics
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate_pct: f64,
    pub avg_win_pct: f64,
    pub avg_loss_pct: f64,
    pub profit_factor: f64,             // Gross profit / Gross loss
    
    // Cost analysis
    pub total_fees_paid: f64,
    pub total_slippage_cost: f64,
    pub cost_per_trade: f64,
    pub cost_as_pct_of_returns: f64,
    
    // Grid-specific metrics
    pub grid_efficiency: f64,           // % of grid levels that triggered
    pub avg_time_in_position_hours: f64,
    pub market_state_distribution: std::collections::HashMap<MarketState, f64>,
}

#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub performance_metrics: PerformanceMetrics,
    pub trades: Vec<Trade>,
    pub equity_curve: Array1<f64>,
    pub timestamps: Vec<DateTime<Utc>>,
    pub grid_statistics: GridStatistics,
    pub market_state_history: Vec<MarketState>,
    
    // Configuration used
    pub trading_pair: String,
    pub timeframe: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_capital: f64,
}

#[derive(Debug, Clone)]
pub struct GridStatistics {
    pub total_grid_setups: usize,
    pub avg_grid_spacing: f64,
    pub grid_spacing_std: f64,
    pub levels_per_setup: Vec<usize>,
    pub adaptation_frequency: f64,      // How often grid was readjusted
    pub state_based_adjustments: usize, // Adjustments due to market state changes
}

#[derive(Debug, Clone)]
pub struct BacktestConfig {
    // Strategy parameters
    pub initial_capital: f64,
    pub grid_levels: usize,
    pub base_grid_spacing: f64,
    
    // Market analysis
    pub price_history_size: usize,
    pub trend_threshold: f64,
    pub volatility_threshold: f64,
    
    // Cost modeling
    pub trading_costs: TradingCosts,
    pub slippage_model: SlippageModel,
    pub risk_config: RiskConfig,
    
    // Markov chain parameters
    pub use_markov_predictions: bool,
    pub markov_lookback_periods: usize,
    pub state_transition_smoothing: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_capital: 10000.0,       // £10k starting capital
            grid_levels: 5,
            base_grid_spacing: 0.01,        // 1% base spacing
            
            price_history_size: 20,
            trend_threshold: 0.005,         // 0.5%
            volatility_threshold: 0.02,     // 2%
            
            trading_costs: TradingCosts::default(),
            slippage_model: SlippageModel::default(),
            risk_config: RiskConfig::default(),
            
            use_markov_predictions: true,
            markov_lookback_periods: 50,
            state_transition_smoothing: 0.1,
        }
    }
}