// Vectorized Price Processing and Grid Signal Generation

use ndarray::{Array1, Array2, s};
use rayon::prelude::*;
use crate::core::types::MarketState;
use crate::backtesting::{HistoricalData, BacktestConfig, TradeType};
use crate::backtesting::markov::MarkovChainAnalyzer;

#[derive(Debug, Clone)]
pub struct VectorizedGridProcessor {
    config: BacktestConfig,
    markov_analyzer: Option<MarkovChainAnalyzer>,
}

impl VectorizedGridProcessor {
    pub fn new(config: BacktestConfig) -> Self {
        let markov_analyzer = if config.use_markov_predictions {
            Some(MarkovChainAnalyzer::new(&config))
        } else {
            None
        };

        Self {
            config,
            markov_analyzer,
        }
    }

    /// Vectorized market state detection across entire price series
    pub fn detect_market_states_vectorized(&mut self, data: &HistoricalData) -> Vec<MarketState> {
        let prices = &data.prices;
        let window_size = self.config.price_history_size;
        let mut states = Vec::with_capacity(prices.len());
        
        // Process in parallel chunks for efficiency
        let _chunk_size = 1000.min(prices.len() / rayon::current_num_threads().max(1));
        
        for i in 0..prices.len() {
            let start_idx = i.saturating_sub(window_size);
            let end_idx = (i + 1).min(prices.len());
            
            if end_idx - start_idx >= 5 { // Need minimum 5 points for state detection
                let window_prices = prices.slice(s![start_idx..end_idx]);
                let state = self.detect_single_market_state(&window_prices);
                states.push(state);
                
                // Update Markov analyzer if enabled
                if let Some(ref mut analyzer) = self.markov_analyzer {
                    analyzer.update_with_state(state);
                }
            } else {
                states.push(MarketState::Ranging); // Default for insufficient data
            }
        }
        
        states
    }

    fn detect_single_market_state(&self, prices: &ndarray::ArrayView1<f64>) -> MarketState {
        if prices.len() < 2 {
            return MarketState::Ranging;
        }

        let first_price = prices[0];
        let last_price = prices[prices.len() - 1];
        let price_change_pct = (last_price - first_price) / first_price;
        
        // Calculate volatility (coefficient of variation)
        let mean_price = prices.mean().unwrap_or(0.0);
        let variance = prices.iter()
            .map(|&p| (p - mean_price).powi(2))
            .sum::<f64>() / prices.len() as f64;
        let volatility = variance.sqrt() / mean_price;
        
        // State detection using thresholds
        if price_change_pct > self.config.trend_threshold && volatility < self.config.volatility_threshold {
            MarketState::TrendingUp
        } else if price_change_pct < -self.config.trend_threshold && volatility < self.config.volatility_threshold {
            MarketState::TrendingDown
        } else {
            MarketState::Ranging
        }
    }

    /// Vectorized grid level computation for entire price series
    pub fn compute_grid_levels_vectorized(&mut self, data: &HistoricalData, states: &[MarketState]) -> GridLevelsResult {
        let prices = &data.prices;
        let n_points = prices.len();
        let n_levels = self.config.grid_levels;
        
        // Pre-allocate arrays for all grid levels
        let mut buy_levels = Array2::<f64>::zeros((n_points, n_levels));
        let mut sell_levels = Array2::<f64>::zeros((n_points, n_levels));
        let mut grid_spacings = Array1::<f64>::zeros(n_points);
        
        // Process in parallel for efficiency
        let _chunk_size = 100.min(n_points / rayon::current_num_threads().max(1));
        
        for (i, (&price, &state)) in prices.iter().zip(states.iter()).enumerate() {
            let adaptive_spacing = self.get_adaptive_spacing(price, state, i);
            grid_spacings[i] = adaptive_spacing;
            
            // Generate buy levels (below current price)
            for level in 0..n_levels {
                let buy_price = price - ((level + 1) as f64 * adaptive_spacing);
                buy_levels[[i, level]] = buy_price;
            }
            
            // Generate sell levels (above current price)
            for level in 0..n_levels {
                let sell_price = price + ((level + 1) as f64 * adaptive_spacing);
                sell_levels[[i, level]] = sell_price;
            }
        }
        
        GridLevelsResult {
            buy_levels,
            sell_levels,
            grid_spacings,
            timestamps: data.timestamps.clone(),
        }
    }

    fn get_adaptive_spacing(&mut self, current_price: f64, current_state: MarketState, _index: usize) -> f64 {
        let base_spacing = self.config.base_grid_spacing * current_price;
        
        // Use Markov predictions if available
        if let Some(ref analyzer) = self.markov_analyzer {
            analyzer.get_adaptive_grid_spacing(base_spacing, current_state)
        } else {
            // Fallback to simple state-based adjustment
            match current_state {
                MarketState::TrendingUp | MarketState::TrendingDown => base_spacing * 1.5,
                MarketState::Ranging => base_spacing,
            }
        }
    }

    /// Vectorized signal detection across entire price series
    pub fn detect_signals_vectorized(&self, data: &HistoricalData, grid_levels: &GridLevelsResult) -> Vec<GridSignalEvent> {
        let prices = &data.prices;
        let buy_levels = &grid_levels.buy_levels;
        let sell_levels = &grid_levels.sell_levels;
        
        let mut signals = Vec::new();
        let mut last_triggered_level: Option<f64> = None;
        
        // Use the FIRST price point's grid levels as fixed levels for the entire backtest
        // This is the correct grid trading approach - levels don't change during the trade
        let mut fixed_buy_levels = Vec::new();
        let mut fixed_sell_levels = Vec::new();
        
        for level_idx in 0..self.config.grid_levels {
            fixed_buy_levels.push(buy_levels[[0, level_idx]]);
            fixed_sell_levels.push(sell_levels[[0, level_idx]]);
        }
        
        for i in 0..prices.len() {
            let current_price = prices[i];
            let timestamp = data.timestamps[i];
            
            // Check buy levels (price crossing below) - using FIXED levels
            for &buy_level in fixed_buy_levels.iter().take(self.config.grid_levels) {
                
                if current_price <= buy_level && last_triggered_level != Some(buy_level) {
                    signals.push(GridSignalEvent {
                        signal_type: TradeType::Buy,
                        price: current_price,
                        grid_level: buy_level,
                        timestamp,
                        index: i,
                        market_state: None, // Will be filled later if needed
                    });
                    last_triggered_level = Some(buy_level);
                    break; // Only one signal per price update
                }
            }
            
            // Check sell levels (price crossing above) - using FIXED levels
            for &sell_level in fixed_sell_levels.iter().take(self.config.grid_levels) {
                
                if current_price >= sell_level && last_triggered_level != Some(sell_level) {
                    signals.push(GridSignalEvent {
                        signal_type: TradeType::Sell,
                        price: current_price,
                        grid_level: sell_level,
                        timestamp,
                        index: i,
                        market_state: None,
                    });
                    last_triggered_level = Some(sell_level);
                    break; // Only one signal per price update
                }
            }
        }
        
        signals
    }

    /// Vectorized cost calculation for all trades
    pub fn calculate_trading_costs_vectorized(&self, signals: &[GridSignalEvent], data: &HistoricalData) -> Vec<TradeCostAnalysis> {
        signals.par_iter().map(|signal| {
            let volume = if signal.index < data.volumes.len() {
                data.volumes[signal.index]
            } else {
                1000.0 // Default volume
            };
            
            self.calculate_single_trade_cost(signal, volume)
        }).collect()
    }

    fn calculate_single_trade_cost(&self, signal: &GridSignalEvent, volume: f64) -> TradeCostAnalysis {
        let costs = &self.config.trading_costs;
        let slippage_model = &self.config.slippage_model;
        
        // Calculate fee (assuming market order for simplicity, could be enhanced for limit orders)
        let fee_rate = costs.taker_fee_rate; // Conservative assumption
        let quantity = 100.0; // Default quantity, could be dynamic
        let fees = signal.price * quantity * fee_rate;
        
        // Calculate slippage
        let base_slippage = signal.price * slippage_model.base_slippage_bps / 10000.0;
        let market_impact = quantity * slippage_model.market_impact_factor;
        let volume_impact = if volume > 0.0 {
            slippage_model.liquidity_factor / volume.sqrt()
        } else {
            slippage_model.liquidity_factor
        };
        
        let total_slippage = base_slippage + market_impact + volume_impact;
        
        let execution_price = match signal.signal_type {
            TradeType::Buy => signal.price + total_slippage,
            TradeType::Sell => signal.price - total_slippage,
        };
        
        TradeCostAnalysis {
            intended_price: signal.price,
            execution_price,
            fees,
            slippage_cost: total_slippage,
            total_cost: fees + total_slippage,
            quantity,
        }
    }

    pub fn get_markov_statistics(&self) -> Option<crate::backtesting::markov::MarkovStatistics> {
        self.markov_analyzer.as_ref().map(|analyzer| analyzer.get_statistics())
    }
}

#[derive(Debug, Clone)]
pub struct GridLevelsResult {
    pub buy_levels: Array2<f64>,      // [time_index, level_index]
    pub sell_levels: Array2<f64>,     // [time_index, level_index]
    pub grid_spacings: Array1<f64>,   // [time_index]
    pub timestamps: Vec<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct GridSignalEvent {
    pub signal_type: TradeType,
    pub price: f64,
    pub grid_level: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub index: usize,
    pub market_state: Option<MarketState>,
}

#[derive(Debug, Clone)]
pub struct TradeCostAnalysis {
    pub intended_price: f64,
    pub execution_price: f64,
    pub fees: f64,
    pub slippage_cost: f64,
    pub total_cost: f64,
    pub quantity: f64,
}

/// Parallel portfolio simulation for multiple parameter combinations
pub fn simulate_multiple_strategies(
    data: &HistoricalData,
    _base_config: &BacktestConfig,
    parameter_grid: &ParameterGrid,
) -> Vec<StrategyResult> {
    parameter_grid.configurations.par_iter().map(|config| {
        let mut processor = VectorizedGridProcessor::new(config.clone());
        
        // Run full vectorized backtest
        let states = processor.detect_market_states_vectorized(data);
        let grid_levels = processor.compute_grid_levels_vectorized(data, &states);
        let signals = processor.detect_signals_vectorized(data, &grid_levels);
        let costs = processor.calculate_trading_costs_vectorized(&signals, data);
        
        // Calculate basic performance metrics
        let total_return = calculate_strategy_return(&signals, &costs, config.initial_capital);
        let num_trades = signals.len();
        
        StrategyResult {
            config: config.clone(),
            total_return,
            num_trades,
            signals: signals.len(),
            markov_stats: processor.get_markov_statistics(),
        }
    }).collect()
}

#[derive(Debug, Clone)]
pub struct ParameterGrid {
    pub configurations: Vec<BacktestConfig>,
}

impl Default for ParameterGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl ParameterGrid {
    pub fn new() -> Self {
        Self {
            configurations: Vec::new(),
        }
    }
    
    pub fn add_grid_spacing_sweep(&mut self, base_config: &BacktestConfig, spacings: &[f64]) {
        for &spacing in spacings {
            let mut config = base_config.clone();
            config.base_grid_spacing = spacing;
            self.configurations.push(config);
        }
    }
    
    pub fn add_grid_levels_sweep(&mut self, base_config: &BacktestConfig, levels: &[usize]) {
        for &level_count in levels {
            let mut config = base_config.clone();
            config.grid_levels = level_count;
            self.configurations.push(config);
        }
    }
}

#[derive(Debug, Clone)]
pub struct StrategyResult {
    pub config: BacktestConfig,
    pub total_return: f64,
    pub num_trades: usize,
    pub signals: usize,
    pub markov_stats: Option<crate::backtesting::markov::MarkovStatistics>,
}

fn calculate_strategy_return(signals: &[GridSignalEvent], costs: &[TradeCostAnalysis], initial_capital: f64) -> f64 {
    let mut capital = initial_capital;
    let mut position = 0.0;
    
    for (signal, cost) in signals.iter().zip(costs.iter()) {
        match signal.signal_type {
            TradeType::Buy => {
                let quantity = cost.quantity;
                capital -= cost.execution_price * quantity + cost.total_cost;
                position += quantity;
            },
            TradeType::Sell => {
                let quantity = cost.quantity.min(position); // Can't sell more than we have
                capital += cost.execution_price * quantity - cost.total_cost;
                position -= quantity;
            },
        }
    }
    
    // Mark to market any remaining position
    if position > 0.0 && !signals.is_empty() {
        let last_price = signals.last().unwrap().price;
        capital += position * last_price;
    }
    
    (capital - initial_capital) / initial_capital * 100.0 // Return as percentage
}