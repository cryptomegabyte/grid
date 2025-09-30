// Main Backtesting Engine that orchestrates all components

use crate::backtesting::{
    BacktestConfig, BacktestResult, GridStatistics, 
    HistoricalData, Trade, TradeType
};
use crate::clients::kraken_api::{KrakenHistoricalClient, KrakenApiError};
use crate::backtesting::vectorized::{
    VectorizedGridProcessor, GridSignalEvent, ParameterGrid, StrategyResult,
    simulate_multiple_strategies, TradeCostAnalysis
};
use crate::backtesting::analytics::PerformanceAnalyzer;
use crate::core::types::MarketState;
use chrono::{DateTime, Utc};
use ndarray::Array1;
// use rayon::prelude::*; // Unused for now
use std::collections::HashMap;


pub struct BacktestingEngine {
    config: BacktestConfig,
    kraken_client: KrakenHistoricalClient,
    performance_analyzer: PerformanceAnalyzer,
}

impl BacktestingEngine {
    pub fn new(config: BacktestConfig) -> Self {
        Self {
            config,
            kraken_client: KrakenHistoricalClient::new(),
            performance_analyzer: PerformanceAnalyzer::new(),
        }
    }

    /// Run a complete backtest for a single trading pair
    pub async fn run_backtest(
        &mut self,
        trading_pair: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        timeframe_minutes: u32,
    ) -> Result<BacktestResult, BacktestError> {
        println!("🚀 Starting backtest for {} from {} to {}", 
                 trading_pair, start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));

        // Fetch historical data
        let historical_data = self.fetch_historical_data(trading_pair, timeframe_minutes, Some(start_date)).await?;
        
        if historical_data.is_empty() {
            return Err(BacktestError::InsufficientData("No historical data available".to_string()));
        }

        println!("📊 Loaded {} price points", historical_data.len());

        // Run vectorized backtest
        self.run_backtest_with_data(&historical_data, trading_pair, start_date, end_date).await
    }

    /// Run backtest with pre-loaded data
    pub async fn run_backtest_with_data(
        &mut self,
        data: &HistoricalData,
        trading_pair: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<BacktestResult, BacktestError> {
        let mut processor = VectorizedGridProcessor::new(self.config.clone());

        // Step 1: Vectorized market state detection
        println!("🔍 Detecting market states...");
        let market_states = processor.detect_market_states_vectorized(data);

        // Step 2: Compute grid levels for entire series
        println!("📏 Computing grid levels...");
        let grid_levels = processor.compute_grid_levels_vectorized(data, &market_states);

        // Step 3: Detect all trading signals
        println!("⚡ Detecting trading signals...");
        let signals = processor.detect_signals_vectorized(data, &grid_levels);
        println!("Found {} trading signals", signals.len());

        // Step 4: Calculate trading costs
        println!("💰 Calculating trading costs...");
        let cost_analyses = processor.calculate_trading_costs_vectorized(&signals, data);

        // Step 5: Simulate portfolio and generate trades
        println!("🏦 Simulating portfolio...");
        let trades = self.simulate_portfolio(&signals, &cost_analyses, data);

        // Step 6: Calculate performance metrics
        println!("📈 Analyzing performance...");
        let performance_metrics = self.performance_analyzer.calculate_comprehensive_metrics(
            &trades,
            &data.prices,
            &data.timestamps,
            self.config.initial_capital,
        );

        // Step 7: Calculate grid statistics
        let grid_statistics = self.calculate_grid_statistics(&grid_levels, &market_states);

        // Step 8: Create equity curve
        let equity_curve = self.calculate_equity_curve(&trades, self.config.initial_capital);

        // Get Markov statistics if available
        let markov_stats = processor.get_markov_statistics();
        if let Some(ref stats) = markov_stats {
            println!("🎯 Markov Analysis: {:.1}% confidence, {} samples", 
                     stats.confidence_level * 100.0, stats.total_samples);
        }

        println!("✅ Backtest completed!");
        println!("📊 Total Return: {:.2}%", performance_metrics.total_return_pct);
        println!("📊 Sharpe Ratio: {:.2}", performance_metrics.sharpe_ratio);
        println!("📊 Max Drawdown: {:.2}%", performance_metrics.max_drawdown_pct);
        println!("📊 Total Trades: {}", performance_metrics.total_trades);
        println!("📊 Win Rate: {:.1}%", performance_metrics.win_rate_pct);

        Ok(BacktestResult {
            performance_metrics,
            trades,
            equity_curve,
            timestamps: data.timestamps.clone(),
            grid_statistics,
            market_state_history: market_states,
            trading_pair: trading_pair.to_string(),
            timeframe: data.timeframe.clone(),
            start_date,
            end_date,
            initial_capital: self.config.initial_capital,
        })
    }

    /// Run parameter optimization across multiple configurations
    pub async fn optimize_parameters(
        &mut self,
        trading_pair: &str,
        start_date: DateTime<Utc>,
        _end_date: DateTime<Utc>,
        timeframe_minutes: u32,
        parameter_grid: ParameterGrid,
    ) -> Result<Vec<StrategyResult>, BacktestError> {
        println!("🔧 Starting parameter optimization with {} configurations", 
                 parameter_grid.configurations.len());

        // Fetch data once for all parameter combinations
        let data = self.fetch_historical_data(trading_pair, timeframe_minutes, Some(start_date)).await?;

        // Run parallel optimization
        let results = simulate_multiple_strategies(&data, &self.config, &parameter_grid);

        // Sort by performance (total return)
        let mut sorted_results = results;
        sorted_results.sort_by(|a, b| b.total_return.partial_cmp(&a.total_return).unwrap());

        println!("🏆 Optimization completed!");
        if let Some(best) = sorted_results.first() {
            println!("🥇 Best configuration: {:.2}% return, {} trades", 
                     best.total_return, best.num_trades);
            println!("   Grid levels: {}, Spacing: {:.4}", 
                     best.config.grid_levels, best.config.base_grid_spacing);
        }

        Ok(sorted_results)
    }

    /// Run multi-pair backtest
    pub async fn run_multi_pair_backtest(
        &mut self,
        trading_pairs: &[&str],
        start_date: DateTime<Utc>,
        _end_date: DateTime<Utc>,
        timeframe_minutes: u32,
    ) -> Result<HashMap<String, BacktestResult>, BacktestError> {
        println!("🌍 Starting multi-pair backtest for {} pairs", trading_pairs.len());

        let mut results = HashMap::new();

        for &pair in trading_pairs {
            match self.run_backtest(pair, start_date, _end_date, timeframe_minutes).await {
                Ok(result) => {
                    println!("✅ Completed backtest for {}: {:.2}% return", 
                             pair, result.performance_metrics.total_return_pct);
                    results.insert(pair.to_string(), result);
                }
                Err(e) => {
                    eprintln!("❌ Failed backtest for {}: {:?}", pair, e);
                    // Continue with other pairs
                }
            }
        }

        Ok(results)
    }

    async fn fetch_historical_data(
        &mut self,
        trading_pair: &str,
        timeframe_minutes: u32,
        since: Option<DateTime<Utc>>,
    ) -> Result<HistoricalData, BacktestError> {
        self.kraken_client
            .fetch_ohlc(trading_pair, timeframe_minutes, since)
            .await
            .map_err(BacktestError::KrakenApiError)
    }

    fn simulate_portfolio(
        &self,
        signals: &[GridSignalEvent],
        cost_analyses: &[TradeCostAnalysis],
        _data: &HistoricalData,
    ) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut position_size = 0.0;
        let mut available_capital = self.config.initial_capital;

        let mut rejected_risk = 0;
        let mut rejected_size = 0;
        let mut rejected_capital = 0;
        
        for (signal, cost_analysis) in signals.iter().zip(cost_analyses.iter()) {
            // Risk management checks
            if !self.should_execute_trade(signal, available_capital, position_size) {
                rejected_risk += 1;
                continue;
            }

            let trade_quantity = self.calculate_position_size(signal, available_capital);
            
            if trade_quantity < self.config.trading_costs.min_order_size {
                rejected_size += 1;
                continue; // Skip trades below minimum size
            }

            // Create trade
            let trade = Trade::new(
                signal.signal_type,
                signal.grid_level,
                cost_analysis.execution_price,
                trade_quantity,
                signal.timestamp,
                signal.grid_level,
                cost_analysis.fees,
                cost_analysis.slippage_cost,
            );

            // Update portfolio state
            match signal.signal_type {
                TradeType::Buy => {
                    let total_cost = cost_analysis.execution_price * trade_quantity + cost_analysis.total_cost;
                    if available_capital >= total_cost {
                        available_capital -= total_cost;
                        position_size += trade_quantity;
                        trades.push(trade);
                    } else {
                        rejected_capital += 1;
                    }
                }
                TradeType::Sell => {
                    let sellable_quantity = trade_quantity.min(position_size);
                    if sellable_quantity > 0.0 {
                        let proceeds = cost_analysis.execution_price * sellable_quantity - cost_analysis.total_cost;
                        available_capital += proceeds;
                        position_size -= sellable_quantity;
                        
                        let mut adjusted_trade = trade;
                        adjusted_trade.quantity = sellable_quantity;
                        trades.push(adjusted_trade);
                    }
                }
            }
        }

        if trades.is_empty() && !signals.is_empty() {
            println!("⚠️  No trades executed from {} signals:", signals.len());
            println!("   - Rejected by risk management: {}", rejected_risk);
            println!("   - Rejected due to small size: {}", rejected_size);
            println!("   - Rejected due to insufficient capital: {}", rejected_capital);
            println!("   - Position size calculation: {:.6} * {:.4} = {:.6}", 
                     available_capital, self.config.risk_config.max_position_size_pct,
                     available_capital * self.config.risk_config.max_position_size_pct);
            println!("   - Min order size: {:.2}", self.config.trading_costs.min_order_size);
        }

        trades
    }

    fn should_execute_trade(&self, signal: &GridSignalEvent, available_capital: f64, position_size: f64) -> bool {
        // Basic risk checks
        let position_value = position_size * signal.price;
        let total_portfolio_value = available_capital + position_value;
        
        // Don't exceed maximum position size
        let max_position_value = total_portfolio_value * self.config.risk_config.max_position_size_pct;
        
        match signal.signal_type {
            TradeType::Buy => {
                // Check if we have enough capital and won't exceed position limits
                let trade_quantity = self.calculate_position_size(signal, available_capital);
                let trade_value = signal.price * trade_quantity;
                let result = available_capital >= trade_value && (position_value + trade_value) <= max_position_value;
                
                // Debug first failed trade
                if !result && position_size == 0.0 {
                    println!("⚠️  Trade rejected (first trade debug):");
                    println!("   Signal: {:?} at price £{:.4}", signal.signal_type, signal.price);
                    println!("   Available capital: £{:.2}", available_capital);
                    println!("   Position size: {:.4} units", position_size);
                    println!("   Position value: £{:.2}", position_value);
                    println!("   Total portfolio: £{:.2}", total_portfolio_value);
                    println!("   Max position value: £{:.2}", max_position_value);
                    println!("   Trade quantity: {:.4} units", trade_quantity);
                    println!("   Trade value: £{:.2}", trade_value);
                    println!("   Capital check: {} >= {} = {}", available_capital, trade_value, available_capital >= trade_value);
                    println!("   Position check: {} <= {} = {}", position_value + trade_value, max_position_value, (position_value + trade_value) <= max_position_value);
                }
                
                result
            }
            TradeType::Sell => {
                // Can only sell if we have a position
                position_size > 0.0
            }
        }
    }

    fn calculate_position_size(&self, signal: &GridSignalEvent, available_capital: f64) -> f64 {
        // Calculate position size as percentage of available capital
        let position_value = available_capital * self.config.risk_config.max_position_size_pct;
        
        // Convert dollar amount to quantity (number of units)
        position_value / signal.price
    }

    fn calculate_grid_statistics(
        &self,
        grid_levels: &crate::backtesting::vectorized::GridLevelsResult,
        market_states: &[MarketState],
    ) -> GridStatistics {
        let spacings = &grid_levels.grid_spacings;
        
        let avg_spacing = spacings.mean().unwrap_or(0.0);
        let spacing_variance = spacings.iter()
            .map(|&x| (x - avg_spacing).powi(2))
            .sum::<f64>() / spacings.len() as f64;
        let spacing_std = spacing_variance.sqrt();

        // Count state transitions for adaptation frequency
        let state_changes = market_states.windows(2)
            .filter(|window| window[0] != window[1])
            .count();
        
        let adaptation_frequency = state_changes as f64 / market_states.len() as f64;

        GridStatistics {
            total_grid_setups: market_states.len(),
            avg_grid_spacing: avg_spacing,
            grid_spacing_std: spacing_std,
            levels_per_setup: vec![self.config.grid_levels; market_states.len()],
            adaptation_frequency,
            state_based_adjustments: state_changes,
        }
    }

    fn calculate_equity_curve(&self, trades: &[Trade], initial_capital: f64) -> Array1<f64> {
        let mut equity = vec![initial_capital];
        let mut running_capital = initial_capital;
        let mut position_value = 0.0;

        for trade in trades {
            match trade.trade_type {
                TradeType::Buy => {
                    running_capital -= trade.price * trade.quantity + trade.fees_paid + trade.slippage_cost;
                    position_value += trade.price * trade.quantity;
                }
                TradeType::Sell => {
                    running_capital += trade.price * trade.quantity - trade.fees_paid - trade.slippage_cost;
                    position_value -= trade.price * trade.quantity;
                }
            }
            
            let total_equity = running_capital + position_value;
            equity.push(total_equity);
        }

        Array1::from_vec(equity)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BacktestError {
    #[error("Kraken API error: {0}")]
    KrakenApiError(#[from] KrakenApiError),
    
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Calculation error: {0}")]
    CalculationError(String),
}

/// Builder pattern for easier backtest configuration
pub struct BacktestBuilder {
    config: BacktestConfig,
}

impl BacktestBuilder {
    pub fn new() -> Self {
        Self {
            config: BacktestConfig::default(),
        }
    }

    pub fn with_initial_capital(mut self, capital: f64) -> Self {
        self.config.initial_capital = capital;
        self
    }

    pub fn with_grid_levels(mut self, levels: usize) -> Self {
        self.config.grid_levels = levels;
        self
    }

    pub fn with_grid_spacing(mut self, spacing: f64) -> Self {
        self.config.base_grid_spacing = spacing;
        self
    }

    pub fn with_markov_analysis(mut self, enabled: bool) -> Self {
        self.config.use_markov_predictions = enabled;
        self
    }

    pub fn with_risk_config(mut self, risk_config: crate::backtesting::RiskConfig) -> Self {
        self.config.risk_config = risk_config;
        self
    }

    pub fn build(self) -> BacktestingEngine {
        BacktestingEngine::new(self.config)
    }
}

impl Default for BacktestBuilder {
    fn default() -> Self {
        Self::new()
    }
}