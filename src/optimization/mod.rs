use crate::{BacktestBuilder, BacktestError};
//  // TODO: Use when implementing result storage
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc, Duration as ChronoDuration};
use std::time::Duration;
use tracing::{info, warn, debug};

pub mod grid_optimizer;
pub mod parameter_search;
pub mod risk_optimizer;

/// Configuration space for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub grid_levels: GridLevelRange,
    pub grid_spacing: GridSpacingRange,
    pub timeframes: Vec<u32>,  // minutes
    pub risk_management: RiskManagementRange,
    pub date_ranges: Vec<DateRange>,
    pub optimization_strategy: OptimizationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridLevelRange {
    pub min: usize,
    pub max: usize,
    pub step: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridSpacingRange {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskManagementRange {
    pub max_drawdown: Vec<f64>,
    pub stop_loss: Vec<f64>,
    pub position_size: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    GridSearch,
    RandomSearch { iterations: usize },
    BayesianOptimization { iterations: usize },
    GeneticAlgorithm { population: usize, generations: usize },
}

/// Individual parameter combination to test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSet {
    pub grid_levels: usize,
    pub grid_spacing: f64,
    pub timeframe_minutes: u32,
    pub max_drawdown: f64,
    pub stop_loss: f64,
    pub position_size: f64,
    pub date_range: DateRange,
}

/// Result of testing a parameter set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub parameters: ParameterSet,
    pub backtest_result: BacktestMetrics,
    pub score: f64,  // Composite optimization score
    pub rank: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestMetrics {
    pub total_return: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub total_trades: usize,
    pub avg_trade_duration: f64,
    pub risk_adjusted_return: f64,
}

/// Main optimization orchestrator
pub struct ParameterOptimizer {
    config: OptimizationConfig,
}

impl ParameterOptimizer {
    pub fn new(config: OptimizationConfig) -> Self {
        Self { config }
    }

    /// Run comprehensive optimization for a single trading pair
    pub async fn optimize_pair(
        &self,
        trading_pair: &str,
    ) -> Result<Vec<OptimizationResult>, BacktestError> {
        info!("🔧 Starting parameter optimization for {}", trading_pair);
        
        let parameter_sets = self.generate_parameter_combinations();
        info!("📊 Testing {} parameter combinations", parameter_sets.len());
        
        let mut results = Vec::new();
        
        for (i, params) in parameter_sets.iter().enumerate() {
            debug!("Testing parameter set {}/{}", i + 1, parameter_sets.len());
            
            // Retry mechanism with exponential backoff
            let mut attempts = 0;
            let max_attempts = 3;
            
            loop {
                match self.test_parameter_set(trading_pair, params).await {
                    Ok(result) => {
                        results.push(result);
                        break;
                    }
                    Err(e) => {
                        attempts += 1;
                        if attempts >= max_attempts {
                            warn!("Failed to test parameter set after {} attempts: {}", max_attempts, e);
                            break;
                        } else {
                            let delay = Duration::from_millis(1000 * (2_u64.pow(attempts - 1)));
                            debug!("Retrying parameter test in {}ms (attempt {}/{})", delay.as_millis(), attempts, max_attempts);
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            }
            
            // Progress reporting
            if (i + 1) % 10 == 0 {
                info!("✅ Completed {}/{} parameter tests", i + 1, parameter_sets.len());
            }
        }
        
        // Rank results by composite score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        for (i, result) in results.iter_mut().enumerate() {
            result.rank = i + 1;
        }
        
        info!("🏆 Optimization complete! Best score: {:.4}", 
              results.first().map(|r| r.score).unwrap_or(0.0));
        
        Ok(results)
    }

    /// Generate all parameter combinations to test
    fn generate_parameter_combinations(&self) -> Vec<ParameterSet> {
        match &self.config.optimization_strategy {
            OptimizationStrategy::GridSearch => {
                self.generate_grid_search_combinations()
            }
            OptimizationStrategy::RandomSearch { iterations } => {
                self.generate_random_search_combinations(*iterations)
            }
            OptimizationStrategy::BayesianOptimization { iterations } => {
                // For now, fall back to random search
                // TODO: Implement proper Bayesian optimization
                self.generate_random_search_combinations(*iterations)
            }
            OptimizationStrategy::GeneticAlgorithm { population, generations } => {
                // For now, fall back to random search
                // TODO: Implement genetic algorithm
                self.generate_random_search_combinations(population * generations)
            }
        }
    }

    fn generate_grid_search_combinations(&self) -> Vec<ParameterSet> {
        let mut combinations = Vec::new();
        
        // Generate grid levels
        let mut grid_levels = Vec::new();
        let mut level = self.config.grid_levels.min;
        while level <= self.config.grid_levels.max {
            grid_levels.push(level);
            level += self.config.grid_levels.step;
        }
        
        // Generate grid spacing values
        let mut grid_spacings = Vec::new();
        let mut spacing = self.config.grid_spacing.min;
        while spacing <= self.config.grid_spacing.max {
            grid_spacings.push(spacing);
            spacing += self.config.grid_spacing.step;
        }
        
        // Cartesian product of all parameters
        for &levels in &grid_levels {
            for &spacing in &grid_spacings {
                for &timeframe in &self.config.timeframes {
                    for &max_dd in &self.config.risk_management.max_drawdown {
                        for &stop_loss in &self.config.risk_management.stop_loss {
                            for &pos_size in &self.config.risk_management.position_size {
                                for date_range in &self.config.date_ranges {
                                    combinations.push(ParameterSet {
                                        grid_levels: levels,
                                        grid_spacing: spacing,
                                        timeframe_minutes: timeframe,
                                        max_drawdown: max_dd,
                                        stop_loss,
                                        position_size: pos_size,
                                        date_range: date_range.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        combinations
    }

    fn generate_random_search_combinations(&self, iterations: usize) -> Vec<ParameterSet> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut combinations = Vec::new();
        
        for _ in 0..iterations {
            let grid_levels = rng.gen_range(
                self.config.grid_levels.min..=self.config.grid_levels.max
            );
            
            let grid_spacing = rng.gen_range(
                self.config.grid_spacing.min..=self.config.grid_spacing.max
            );
            
            let timeframe = *self.config.timeframes
                .get(rng.gen_range(0..self.config.timeframes.len()))
                .unwrap();
            
            let max_drawdown = *self.config.risk_management.max_drawdown
                .get(rng.gen_range(0..self.config.risk_management.max_drawdown.len()))
                .unwrap();
            
            let stop_loss = *self.config.risk_management.stop_loss
                .get(rng.gen_range(0..self.config.risk_management.stop_loss.len()))
                .unwrap();
            
            let position_size = *self.config.risk_management.position_size
                .get(rng.gen_range(0..self.config.risk_management.position_size.len()))
                .unwrap();
            
            let date_range = self.config.date_ranges
                .get(rng.gen_range(0..self.config.date_ranges.len()))
                .unwrap()
                .clone();
            
            combinations.push(ParameterSet {
                grid_levels,
                grid_spacing,
                timeframe_minutes: timeframe,
                max_drawdown,
                stop_loss,
                position_size,
                date_range,
            });
        }
        
        combinations
    }

    /// Test a single parameter set
    async fn test_parameter_set(
        &self,
        trading_pair: &str,
        params: &ParameterSet,
    ) -> Result<OptimizationResult, BacktestError> {
        let builder = BacktestBuilder::new()
            .with_grid_levels(params.grid_levels)
            .with_grid_spacing(params.grid_spacing);
        
        let mut engine = builder.build();
        let backtest_result = engine.run_backtest(
            trading_pair,
            params.date_range.start,
            params.date_range.end,
            params.timeframe_minutes,
        ).await?;
        
        let metrics = BacktestMetrics {
            total_return: backtest_result.performance_metrics.total_return_pct,
            sharpe_ratio: backtest_result.performance_metrics.sharpe_ratio,
            max_drawdown: backtest_result.performance_metrics.max_drawdown_pct,
            win_rate: backtest_result.performance_metrics.win_rate_pct,
            profit_factor: backtest_result.performance_metrics.profit_factor,
            total_trades: backtest_result.performance_metrics.total_trades,
            avg_trade_duration: backtest_result.performance_metrics.avg_time_in_position_hours,
            risk_adjusted_return: backtest_result.performance_metrics.total_return_pct 
                / (backtest_result.performance_metrics.volatility_pct.max(0.01)),
        };
        
        let score = self.calculate_composite_score(&metrics);
        
        Ok(OptimizationResult {
            parameters: params.clone(),
            backtest_result: metrics,
            score,
            rank: 0, // Will be set later
        })
    }

    /// Calculate composite optimization score
    fn calculate_composite_score(&self, metrics: &BacktestMetrics) -> f64 {
        // Multi-objective optimization score
        // Weights can be adjusted based on preferences
        let return_weight = 0.3;
        let sharpe_weight = 0.25;
        let drawdown_weight = 0.2;  // Penalty for high drawdown
        let win_rate_weight = 0.15;
        let profit_factor_weight = 0.1;
        
        let return_score = metrics.total_return / 100.0;  // Normalize
        let sharpe_score = metrics.sharpe_ratio.max(0.0) / 3.0;  // Cap at 3
        let drawdown_penalty = 1.0 - (metrics.max_drawdown / 100.0).min(0.5);  // Penalty
        let win_rate_score = metrics.win_rate / 100.0;
        let profit_factor_score = (metrics.profit_factor - 1.0).max(0.0) / 2.0;  // Cap at 3
        
        return_weight * return_score
            + sharpe_weight * sharpe_score
            + drawdown_weight * drawdown_penalty
            + win_rate_weight * win_rate_score
            + profit_factor_weight * profit_factor_score
    }
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        let now = Utc::now();
        
        Self {
            grid_levels: GridLevelRange {
                min: 3,
                max: 15,
                step: 2,
            },
            grid_spacing: GridSpacingRange {
                min: 0.005,
                max: 0.05,
                step: 0.005,
            },
            timeframes: vec![5, 15, 30, 60, 240, 1440],  // 5m to 1d
            risk_management: RiskManagementRange {
                max_drawdown: vec![0.05, 0.10, 0.15, 0.20],
                stop_loss: vec![0.02, 0.05, 0.10],
                position_size: vec![0.1, 0.25, 0.5],
            },
            date_ranges: vec![
                DateRange {
                    start: now - ChronoDuration::days(30),
                    end: now,
                    description: "Last 30 days".to_string(),
                },
                DateRange {
                    start: now - ChronoDuration::days(90),
                    end: now - ChronoDuration::days(30),
                    description: "30-90 days ago".to_string(),
                },
                DateRange {
                    start: now - ChronoDuration::days(180),
                    end: now - ChronoDuration::days(90),
                    description: "90-180 days ago".to_string(),
                },
            ],
            optimization_strategy: OptimizationStrategy::RandomSearch { iterations: 100 },
        }
    }
}