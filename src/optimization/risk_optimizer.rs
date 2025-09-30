use super::*;

/// Risk management optimization engine
pub struct RiskOptimizer {
    pub risk_models: Vec<RiskModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskModel {
    /// Fixed percentage risk per trade
    FixedPercentage { risk_per_trade: f64 },
    
    /// Kelly criterion based position sizing
    KellyCriterion { 
        win_rate: f64,
        avg_win_loss_ratio: f64,
        max_kelly_fraction: f64,
    },
    
    /// Volatility-adjusted position sizing
    VolatilityAdjusted {
        base_risk: f64,
        volatility_lookback: usize,
        volatility_multiplier: f64,
    },
    
    /// Maximum drawdown based risk control
    DrawdownBased {
        max_portfolio_drawdown: f64,
        drawdown_lookback: usize,
        recovery_factor: f64,
    },
    
    /// Value at Risk (VaR) based sizing
    ValueAtRisk {
        confidence_level: f64,  // e.g., 0.95 for 95% VaR
        lookback_periods: usize,
        max_var_percentage: f64,
    },
    
    /// Dynamic risk based on market conditions
    MarketConditionBased {
        base_risk: f64,
        volatility_adjustment: f64,
        trend_adjustment: f64,
        correlation_adjustment: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub max_drawdown: f64,
    pub value_at_risk_95: f64,
    pub conditional_value_at_risk: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
    pub max_consecutive_losses: usize,
    pub recovery_time_days: f64,
    pub tail_ratio: f64,
    pub risk_adjusted_return: f64,
}

#[derive(Debug, Clone)]
pub struct RiskOptimizationResult {
    pub risk_model: RiskModel,
    pub parameters: ParameterSet,
    pub risk_metrics: RiskMetrics,
    pub composite_risk_score: f64,
    pub rank: usize,
}

impl RiskOptimizer {
    pub fn new() -> Self {
        Self {
            risk_models: vec![
                RiskModel::FixedPercentage { risk_per_trade: 0.02 },
                RiskModel::KellyCriterion { 
                    win_rate: 0.6, 
                    avg_win_loss_ratio: 1.5, 
                    max_kelly_fraction: 0.25 
                },
                RiskModel::VolatilityAdjusted {
                    base_risk: 0.02,
                    volatility_lookback: 20,
                    volatility_multiplier: 1.0,
                },
                RiskModel::DrawdownBased {
                    max_portfolio_drawdown: 0.10,
                    drawdown_lookback: 50,
                    recovery_factor: 2.0,
                },
                RiskModel::ValueAtRisk {
                    confidence_level: 0.95,
                    lookback_periods: 50,
                    max_var_percentage: 0.05,
                },
                RiskModel::MarketConditionBased {
                    base_risk: 0.02,
                    volatility_adjustment: 0.5,
                    trend_adjustment: 0.3,
                    correlation_adjustment: 0.2,
                },
            ]
        }
    }

    /// Optimize risk management parameters for a trading pair
    pub async fn optimize_risk_management(
        &self,
        trading_pair: &str,
        base_parameters: &ParameterSet,
        historical_returns: &[f64],
    ) -> Result<Vec<RiskOptimizationResult>, BacktestError> {
        info!("⚖️ Optimizing risk management for {}", trading_pair);
        
        let mut results = Vec::new();
        
        for risk_model in &self.risk_models {
            let optimized_params = self.optimize_for_risk_model(
                base_parameters, 
                risk_model, 
                historical_returns
            ).await?;
            
            let risk_metrics = self.calculate_risk_metrics(historical_returns);
            let composite_score = self.calculate_risk_score(&risk_metrics);
            
            results.push(RiskOptimizationResult {
                risk_model: risk_model.clone(),
                parameters: optimized_params,
                risk_metrics,
                composite_risk_score: composite_score,
                rank: 0,
            });
        }
        
        // Rank by composite risk score (lower is better for risk)
        results.sort_by(|a, b| a.composite_risk_score.partial_cmp(&b.composite_risk_score).unwrap());
        for (i, result) in results.iter_mut().enumerate() {
            result.rank = i + 1;
        }
        
        Ok(results)
    }

    async fn optimize_for_risk_model(
        &self,
        base_params: &ParameterSet,
        risk_model: &RiskModel,
        historical_returns: &[f64],
    ) -> Result<ParameterSet, BacktestError> {
        let mut optimized_params = base_params.clone();
        
        match risk_model {
            RiskModel::FixedPercentage { risk_per_trade } => {
                optimized_params.position_size = *risk_per_trade;
            }
            
            RiskModel::KellyCriterion { win_rate, avg_win_loss_ratio, max_kelly_fraction } => {
                let kelly_fraction = self.calculate_kelly_fraction(*win_rate, *avg_win_loss_ratio);
                optimized_params.position_size = kelly_fraction.min(*max_kelly_fraction);
            }
            
            RiskModel::VolatilityAdjusted { base_risk, volatility_lookback, volatility_multiplier } => {
                let volatility = self.calculate_volatility(historical_returns, *volatility_lookback);
                optimized_params.position_size = base_risk * (1.0 + volatility * volatility_multiplier);
            }
            
            RiskModel::DrawdownBased { max_portfolio_drawdown,                     drawdown_lookback, recovery_factor: _ } => {
                let current_drawdown = self.calculate_current_drawdown(historical_returns, *drawdown_lookback);
                let risk_adjustment = if current_drawdown > max_portfolio_drawdown / 2.0 {
                    0.5 // Reduce risk when approaching max drawdown
                } else {
                    1.0
                };
                optimized_params.position_size = base_params.position_size * risk_adjustment;
            }
            
            RiskModel::ValueAtRisk { confidence_level, lookback_periods, max_var_percentage } => {
                let var = self.calculate_value_at_risk(historical_returns, *confidence_level, *lookback_periods);
                let max_position = max_var_percentage / var.abs();
                optimized_params.position_size = optimized_params.position_size.min(max_position);
            }
            
            RiskModel::MarketConditionBased { base_risk, volatility_adjustment, trend_adjustment, correlation_adjustment: _ } => {
                let volatility = self.calculate_volatility(historical_returns, 20);
                let trend_strength = self.calculate_trend_strength(historical_returns);
                
                let volatility_factor = 1.0 + volatility * volatility_adjustment;
                let trend_factor = 1.0 + trend_strength * trend_adjustment;
                
                optimized_params.position_size = base_risk * volatility_factor * trend_factor;
            }
        }
        
        Ok(optimized_params)
    }

    fn calculate_risk_metrics(&self, returns: &[f64]) -> RiskMetrics {
        if returns.is_empty() {
            return RiskMetrics {
                max_drawdown: 0.0,
                value_at_risk_95: 0.0,
                conditional_value_at_risk: 0.0,
                sharpe_ratio: 0.0,
                sortino_ratio: 0.0,
                calmar_ratio: 0.0,
                max_consecutive_losses: 0,
                recovery_time_days: 0.0,
                tail_ratio: 0.0,
                risk_adjusted_return: 0.0,
            };
        }

        let max_drawdown = self.calculate_max_drawdown(returns);
        let var_95 = self.calculate_value_at_risk(returns, 0.95, returns.len());
        let cvar_95 = self.calculate_conditional_var(returns, 0.95);
        let sharpe = self.calculate_sharpe_ratio(returns);
        let sortino = self.calculate_sortino_ratio(returns);
        let calmar = self.calculate_calmar_ratio(returns, max_drawdown);
        let max_consecutive_losses = self.calculate_max_consecutive_losses(returns);
        let recovery_time = self.calculate_recovery_time(returns);
        let tail_ratio = self.calculate_tail_ratio(returns);
        let risk_adjusted_return = self.calculate_risk_adjusted_return(returns);

        RiskMetrics {
            max_drawdown,
            value_at_risk_95: var_95,
            conditional_value_at_risk: cvar_95,
            sharpe_ratio: sharpe,
            sortino_ratio: sortino,
            calmar_ratio: calmar,
            max_consecutive_losses,
            recovery_time_days: recovery_time,
            tail_ratio,
            risk_adjusted_return,
        }
    }

    fn calculate_risk_score(&self, metrics: &RiskMetrics) -> f64 {
        // Lower score is better (less risky)
        // Weighted combination of risk metrics
        let drawdown_penalty = metrics.max_drawdown * 2.0;
        let var_penalty = metrics.value_at_risk_95.abs();
        let consecutive_loss_penalty = metrics.max_consecutive_losses as f64 * 0.1;
        let recovery_penalty = metrics.recovery_time_days / 100.0;
        
        // Benefits (negative penalties)
        let sharpe_benefit = -metrics.sharpe_ratio.max(0.0) * 0.5;
        let calmar_benefit = -metrics.calmar_ratio.max(0.0) * 0.3;
        
        drawdown_penalty + var_penalty + consecutive_loss_penalty + recovery_penalty + sharpe_benefit + calmar_benefit
    }

    // Risk calculation helper methods
    fn calculate_kelly_fraction(&self, win_rate: f64, avg_win_loss_ratio: f64) -> f64 {
        // Kelly formula: f = (bp - q) / b
        // where b = avg_win_loss_ratio, p = win_rate, q = 1 - win_rate
        let b = avg_win_loss_ratio;
        let p = win_rate;
        let q = 1.0 - win_rate;
        
        ((b * p) - q) / b
    }

    fn calculate_volatility(&self, returns: &[f64], lookback: usize) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let start_idx = if returns.len() > lookback {
            returns.len() - lookback
        } else {
            0
        };
        
        let recent_returns = &returns[start_idx..];
        let mean = recent_returns.iter().sum::<f64>() / recent_returns.len() as f64;
        
        let variance = recent_returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / recent_returns.len() as f64;
            
        variance.sqrt()
    }

    fn calculate_max_drawdown(&self, returns: &[f64]) -> f64 {
        let mut peak = 0.0;
        let mut max_drawdown = 0.0;
        let mut cumulative = 0.0;
        
        for &ret in returns {
            cumulative += ret;
            if cumulative > peak {
                peak = cumulative;
            }
            let drawdown = (peak - cumulative) / peak.max(1.0);
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
        
        max_drawdown
    }

    fn calculate_current_drawdown(&self, returns: &[f64], lookback: usize) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let start_idx = if returns.len() > lookback {
            returns.len() - lookback
        } else {
            0
        };
        
        self.calculate_max_drawdown(&returns[start_idx..])
    }

    fn calculate_value_at_risk(&self, returns: &[f64], confidence_level: f64, lookback: usize) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let start_idx = if returns.len() > lookback {
            returns.len() - lookback
        } else {
            0
        };
        
        let mut recent_returns = returns[start_idx..].to_vec();
        recent_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((1.0 - confidence_level) * recent_returns.len() as f64) as usize;
        recent_returns[index.min(recent_returns.len() - 1)]
    }

    fn calculate_conditional_var(&self, returns: &[f64], confidence_level: f64) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let var = self.calculate_value_at_risk(returns, confidence_level, returns.len());
        let tail_returns: Vec<f64> = returns.iter()
            .filter(|&&r| r <= var)
            .cloned()
            .collect();
            
        if tail_returns.is_empty() {
            var
        } else {
            tail_returns.iter().sum::<f64>() / tail_returns.len() as f64
        }
    }

    fn calculate_sharpe_ratio(&self, returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let volatility = self.calculate_volatility(returns, returns.len());
        
        if volatility == 0.0 {
            0.0
        } else {
            mean_return / volatility
        }
    }

    fn calculate_sortino_ratio(&self, returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let negative_returns: Vec<f64> = returns.iter()
            .filter(|&&r| r < 0.0)
            .cloned()
            .collect();
            
        if negative_returns.is_empty() {
            f64::INFINITY
        } else {
            let downside_deviation = self.calculate_volatility(&negative_returns, negative_returns.len());
            if downside_deviation == 0.0 {
                0.0
            } else {
                mean_return / downside_deviation
            }
        }
    }

    fn calculate_calmar_ratio(&self, returns: &[f64], max_drawdown: f64) -> f64 {
        if returns.is_empty() || max_drawdown == 0.0 {
            return 0.0;
        }
        
        let annual_return = returns.iter().sum::<f64>() * 252.0 / returns.len() as f64; // Assuming daily returns
        annual_return / max_drawdown
    }

    fn calculate_max_consecutive_losses(&self, returns: &[f64]) -> usize {
        let mut max_consecutive = 0;
        let mut current_consecutive = 0;
        
        for &ret in returns {
            if ret < 0.0 {
                current_consecutive += 1;
                max_consecutive = max_consecutive.max(current_consecutive);
            } else {
                current_consecutive = 0;
            }
        }
        
        max_consecutive
    }

    fn calculate_recovery_time(&self, returns: &[f64]) -> f64 {
        // Simplified recovery time calculation
        // Time to recover from maximum drawdown
        if returns.is_empty() {
            return 0.0;
        }
        
        let mut peak = 0.0;
        let mut cumulative = 0.0;
        let mut in_drawdown = false;
        let mut drawdown_start = 0;
        let mut max_recovery_time = 0;
        
        for (i, &ret) in returns.iter().enumerate() {
            cumulative += ret;
            
            if cumulative > peak {
                if in_drawdown {
                    // Recovered from drawdown
                    let recovery_time = i - drawdown_start;
                    max_recovery_time = max_recovery_time.max(recovery_time);
                    in_drawdown = false;
                }
                peak = cumulative;
            } else if !in_drawdown && cumulative < peak * 0.95 {
                // Entered drawdown (5% drop from peak)
                in_drawdown = true;
                drawdown_start = i;
            }
        }
        
        max_recovery_time as f64
    }

    fn calculate_tail_ratio(&self, returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let var_95 = self.calculate_value_at_risk(returns, 0.95, returns.len());
        let var_5 = self.calculate_value_at_risk(returns, 0.05, returns.len());
        
        if var_5 == 0.0 {
            0.0
        } else {
            var_95.abs() / var_5.abs()
        }
    }

    fn calculate_risk_adjusted_return(&self, returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let total_return = returns.iter().sum::<f64>();
        let max_drawdown = self.calculate_max_drawdown(returns);
        
        if max_drawdown == 0.0 {
            total_return
        } else {
            total_return / (1.0 + max_drawdown)
        }
    }

    fn calculate_trend_strength(&self, returns: &[f64]) -> f64 {
        if returns.len() < 10 {
            return 0.0;
        }
        
        // Simple trend strength using linear regression slope
        let n = returns.len() as f64;
        let x_sum = (0..returns.len()).sum::<usize>() as f64;
        let y_sum = returns.iter().sum::<f64>();
        let xy_sum = returns.iter().enumerate()
            .map(|(i, &r)| i as f64 * r)
            .sum::<f64>();
        let x_squared_sum = (0..returns.len())
            .map(|i| (i as f64).powi(2))
            .sum::<f64>();
        
        let slope = (n * xy_sum - x_sum * y_sum) / (n * x_squared_sum - x_sum.powi(2));
        slope.abs()
    }
}

impl Default for RiskOptimizer {
    fn default() -> Self {
        Self::new()
    }
}