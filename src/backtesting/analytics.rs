// Performance Analytics and Metrics Calculation

use crate::backtesting::{Trade, PerformanceMetrics, TradeType};
use ndarray::Array1;
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

pub struct PerformanceAnalyzer {
    risk_free_rate: f64, // Annual risk-free rate for Sharpe calculation
}

impl PerformanceAnalyzer {
    pub fn new() -> Self {
        Self {
            risk_free_rate: 0.02, // 2% annual risk-free rate
        }
    }

    pub fn with_risk_free_rate(mut self, rate: f64) -> Self {
        self.risk_free_rate = rate;
        self
    }

    /// Calculate comprehensive performance metrics
    pub fn calculate_comprehensive_metrics(
        &self,
        trades: &[Trade],
        _prices: &Array1<f64>,
        timestamps: &[DateTime<Utc>],
        initial_capital: f64,
    ) -> PerformanceMetrics {
        if trades.is_empty() {
            return self.empty_metrics();
        }

        // Calculate returns
        let (total_return_pct, annualized_return_pct) = self.calculate_returns(trades, timestamps, initial_capital);
        
        // Calculate risk metrics
        let equity_curve = self.build_equity_curve(trades, initial_capital);
        let returns = self.calculate_return_series(&equity_curve);
        let volatility_pct = self.calculate_volatility(&returns, timestamps) * 100.0;
        let sharpe_ratio = self.calculate_sharpe_ratio(annualized_return_pct, volatility_pct);
        let max_drawdown_pct = self.calculate_max_drawdown(&equity_curve) * 100.0;
        let (var_95, cvar_95) = self.calculate_var_and_cvar(&returns, 0.05);
        
        // Calculate trading statistics
        let (winning_trades, losing_trades, avg_win_pct, avg_loss_pct, profit_factor) = 
            self.calculate_trading_stats(trades);
        
        let win_rate_pct = if trades.len() > 0 {
            winning_trades as f64 / trades.len() as f64 * 100.0
        } else {
            0.0
        };
        
        // Calculate cost analysis
        let (total_fees, total_slippage, cost_per_trade, cost_pct_of_returns) = 
            self.calculate_cost_analysis(trades, total_return_pct, initial_capital);
        
        // Grid-specific metrics (placeholder - would need more data)
        let grid_efficiency = self.calculate_grid_efficiency(trades);
        let avg_time_in_position = self.calculate_avg_time_in_position(trades);
        
        // Market state distribution (placeholder)
        let market_state_distribution = HashMap::new();
        
        PerformanceMetrics {
            total_return_pct,
            annualized_return_pct,
            excess_return_pct: annualized_return_pct - self.risk_free_rate * 100.0,
            volatility_pct,
            sharpe_ratio,
            max_drawdown_pct,
            value_at_risk_95: var_95 * 100.0,
            conditional_var_95: cvar_95 * 100.0,
            total_trades: trades.len(),
            winning_trades,
            losing_trades,
            win_rate_pct,
            avg_win_pct,
            avg_loss_pct,
            profit_factor,
            total_fees_paid: total_fees,
            total_slippage_cost: total_slippage,
            cost_per_trade,
            cost_as_pct_of_returns: cost_pct_of_returns,
            grid_efficiency,
            avg_time_in_position_hours: avg_time_in_position,
            market_state_distribution,
        }
    }

    fn calculate_returns(
        &self,
        trades: &[Trade],
        timestamps: &[DateTime<Utc>],
        initial_capital: f64,
    ) -> (f64, f64) {
        let final_value = self.calculate_final_portfolio_value(trades, initial_capital);
        let total_return_pct = (final_value - initial_capital) / initial_capital * 100.0;
        
        // Calculate annualized return
        let days = if let (Some(first), Some(last)) = (timestamps.first(), timestamps.last()) {
            (*last - *first).num_days().max(1) as f64
        } else {
            365.0 // Default to 1 year if no timestamps
        };
        
        let years = days / 365.25;
        let annualized_return_pct = if years > 0.0 {
            ((final_value / initial_capital).powf(1.0 / years) - 1.0) * 100.0
        } else {
            total_return_pct
        };
        
        (total_return_pct, annualized_return_pct)
    }

    fn calculate_final_portfolio_value(&self, trades: &[Trade], initial_capital: f64) -> f64 {
        let mut cash = initial_capital;
        let mut position_quantity = 0.0;
        let mut last_price = 0.0;

        for trade in trades {
            last_price = trade.price;
            match trade.trade_type {
                TradeType::Buy => {
                    cash -= trade.price * trade.quantity + trade.fees_paid + trade.slippage_cost;
                    position_quantity += trade.quantity;
                }
                TradeType::Sell => {
                    cash += trade.price * trade.quantity - trade.fees_paid - trade.slippage_cost;
                    position_quantity -= trade.quantity;
                }
            }
        }

        // Mark to market any remaining position
        cash + position_quantity * last_price
    }

    fn build_equity_curve(&self, trades: &[Trade], initial_capital: f64) -> Vec<f64> {
        let mut equity_curve = vec![initial_capital];
        let mut cash = initial_capital;
        let mut position_quantity = 0.0;

        for trade in trades {
            match trade.trade_type {
                TradeType::Buy => {
                    cash -= trade.price * trade.quantity + trade.fees_paid + trade.slippage_cost;
                    position_quantity += trade.quantity;
                }
                TradeType::Sell => {
                    cash += trade.price * trade.quantity - trade.fees_paid - trade.slippage_cost;
                    position_quantity -= trade.quantity;
                }
            }
            
            let total_value = cash + position_quantity * trade.price;
            equity_curve.push(total_value);
        }

        equity_curve
    }

    fn calculate_return_series(&self, equity_curve: &[f64]) -> Vec<f64> {
        equity_curve
            .windows(2)
            .map(|window| (window[1] - window[0]) / window[0])
            .collect()
    }

    fn calculate_volatility(&self, returns: &[f64], timestamps: &[DateTime<Utc>]) -> f64 {
        if returns.len() < 2 {
            return 0.0;
        }

        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns.iter()
            .map(|&r| (r - mean_return).powi(2))
            .sum::<f64>() / (returns.len() - 1) as f64;
        
        let std_dev = variance.sqrt();
        
        // Annualize volatility based on data frequency
        let annualization_factor = self.get_annualization_factor(timestamps);
        std_dev * annualization_factor.sqrt()
    }

    fn get_annualization_factor(&self, timestamps: &[DateTime<Utc>]) -> f64 {
        if timestamps.len() < 2 {
            return 252.0; // Default to daily
        }
        
        let avg_interval = timestamps
            .windows(2)
            .map(|window| (window[1] - window[0]).num_minutes())
            .sum::<i64>() / (timestamps.len() - 1) as i64;
        
        match avg_interval {
            0..=5 => 252.0 * 24.0 * 12.0,     // 5-minute data
            6..=15 => 252.0 * 24.0 * 4.0,     // 15-minute data
            16..=60 => 252.0 * 24.0,          // Hourly data
            61..=1440 => 252.0,               // Daily data
            _ => 52.0,                        // Weekly or less frequent
        }
    }

    fn calculate_sharpe_ratio(&self, annualized_return_pct: f64, volatility_pct: f64) -> f64 {
        if volatility_pct <= 0.0 {
            return 0.0;
        }
        
        let excess_return = annualized_return_pct - self.risk_free_rate * 100.0;
        excess_return / volatility_pct
    }

    fn calculate_max_drawdown(&self, equity_curve: &[f64]) -> f64 {
        let mut max_drawdown: f64 = 0.0;
        let mut peak = equity_curve[0];

        for &value in equity_curve.iter().skip(1) {
            if value > peak {
                peak = value;
            } else {
                let drawdown = (peak - value) / peak;
                max_drawdown = max_drawdown.max(drawdown);
            }
        }

        max_drawdown
    }

    fn calculate_var_and_cvar(&self, returns: &[f64], confidence_level: f64) -> (f64, f64) {
        if returns.is_empty() {
            return (0.0, 0.0);
        }

        let mut sorted_returns = returns.to_vec();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let var_index = ((1.0 - confidence_level) * returns.len() as f64) as usize;
        let var = if var_index < sorted_returns.len() {
            -sorted_returns[var_index]
        } else {
            0.0
        };

        // Conditional VaR (Expected Shortfall)
        let cvar = if var_index > 0 {
            let tail_returns: f64 = sorted_returns.iter().take(var_index).sum();
            -tail_returns / var_index as f64
        } else {
            var
        };

        (var, cvar)
    }

    fn calculate_trading_stats(&self, trades: &[Trade]) -> (usize, usize, f64, f64, f64) {
        let mut winning_trades = 0;
        let mut losing_trades = 0;
        let mut total_wins = 0.0;
        let mut total_losses = 0.0;
        let mut gross_profit = 0.0;
        let mut gross_loss = 0.0;

        for trade in trades {
            if trade.net_pnl > 0.0 {
                winning_trades += 1;
                total_wins += trade.net_pnl;
                gross_profit += trade.net_pnl;
            } else if trade.net_pnl < 0.0 {
                losing_trades += 1;
                total_losses += trade.net_pnl.abs();
                gross_loss += trade.net_pnl.abs();
            }
        }

        let avg_win_pct = if winning_trades > 0 {
            (total_wins / winning_trades as f64) * 100.0
        } else {
            0.0
        };

        let avg_loss_pct = if losing_trades > 0 {
            (total_losses / losing_trades as f64) * 100.0
        } else {
            0.0
        };

        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };

        (winning_trades, losing_trades, avg_win_pct, avg_loss_pct, profit_factor)
    }

    fn calculate_cost_analysis(&self, trades: &[Trade], total_return_pct: f64, initial_capital: f64) -> (f64, f64, f64, f64) {
        let total_fees: f64 = trades.iter().map(|t| t.fees_paid).sum();
        let total_slippage: f64 = trades.iter().map(|t| t.slippage_cost).sum();
        
        let cost_per_trade = if !trades.is_empty() {
            (total_fees + total_slippage) / trades.len() as f64
        } else {
            0.0
        };

        let total_returns_value = initial_capital * total_return_pct / 100.0;
        let cost_pct_of_returns = if total_returns_value != 0.0 {
            (total_fees + total_slippage) / total_returns_value.abs() * 100.0
        } else {
            0.0
        };

        (total_fees, total_slippage, cost_per_trade, cost_pct_of_returns)
    }

    fn calculate_grid_efficiency(&self, trades: &[Trade]) -> f64 {
        // Simplified grid efficiency calculation
        // In practice, this would need more context about grid levels
        if trades.is_empty() {
            return 0.0;
        }

        // For now, assume efficiency is related to trade frequency
        // Higher frequency might indicate better grid level utilization
        let unique_levels: std::collections::HashSet<_> = trades
            .iter()
            .map(|t| (t.grid_level * 10000.0) as i32) // Round to avoid floating point issues
            .collect();

        unique_levels.len() as f64 / trades.len() as f64 * 100.0
    }

    fn calculate_avg_time_in_position(&self, trades: &[Trade]) -> f64 {
        if trades.len() < 2 {
            return 0.0;
        }

        let mut total_duration = Duration::zero();
        let mut position_start: Option<DateTime<Utc>> = None;
        let mut position_size = 0.0;

        for trade in trades {
            match trade.trade_type {
                TradeType::Buy => {
                    if position_size == 0.0 {
                        position_start = Some(trade.timestamp);
                    }
                    position_size += trade.quantity;
                }
                TradeType::Sell => {
                    position_size -= trade.quantity;
                    if position_size <= 0.0 {
                        if let Some(start) = position_start {
                            total_duration = total_duration + (trade.timestamp - start);
                        }
                        position_start = None;
                        position_size = 0.0;
                    }
                }
            }
        }

        let num_positions = trades.iter()
            .filter(|t| matches!(t.trade_type, TradeType::Buy))
            .count();

        if num_positions > 0 {
            total_duration.num_hours() as f64 / num_positions as f64
        } else {
            0.0
        }
    }

    fn empty_metrics(&self) -> PerformanceMetrics {
        PerformanceMetrics {
            total_return_pct: 0.0,
            annualized_return_pct: 0.0,
            excess_return_pct: 0.0,
            volatility_pct: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown_pct: 0.0,
            value_at_risk_95: 0.0,
            conditional_var_95: 0.0,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate_pct: 0.0,
            avg_win_pct: 0.0,
            avg_loss_pct: 0.0,
            profit_factor: 0.0,
            total_fees_paid: 0.0,
            total_slippage_cost: 0.0,
            cost_per_trade: 0.0,
            cost_as_pct_of_returns: 0.0,
            grid_efficiency: 0.0,
            avg_time_in_position_hours: 0.0,
            market_state_distribution: HashMap::new(),
        }
    }
}

impl Default for PerformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Additional utility functions for performance analysis
pub fn calculate_information_ratio(returns: &[f64], benchmark_returns: &[f64]) -> f64 {
    if returns.len() != benchmark_returns.len() || returns.is_empty() {
        return 0.0;
    }

    let excess_returns: Vec<f64> = returns
        .iter()
        .zip(benchmark_returns.iter())
        .map(|(&r, &b)| r - b)
        .collect();

    let mean_excess = excess_returns.iter().sum::<f64>() / excess_returns.len() as f64;
    
    if excess_returns.len() < 2 {
        return 0.0;
    }

    let tracking_error = {
        let variance = excess_returns.iter()
            .map(|&r| (r - mean_excess).powi(2))
            .sum::<f64>() / (excess_returns.len() - 1) as f64;
        variance.sqrt()
    };

    if tracking_error > 0.0 {
        mean_excess / tracking_error
    } else {
        0.0
    }
}

/// Calculate Calmar ratio (Annual return / Max drawdown)
pub fn calculate_calmar_ratio(annual_return: f64, max_drawdown: f64) -> f64 {
    if max_drawdown > 0.0 {
        annual_return / max_drawdown
    } else if annual_return > 0.0 {
        f64::INFINITY
    } else {
        0.0
    }
}