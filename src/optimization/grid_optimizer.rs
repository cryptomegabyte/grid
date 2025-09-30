use super::*;


/// Advanced grid strategy configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GridStrategy {
    /// Fixed grid with equal spacing
    Uniform { spacing: f64 },
    
    /// Fibonacci-based grid spacing
    Fibonacci { base_spacing: f64 },
    
    /// Volatility-adjusted dynamic spacing
    VolatilityAdjusted { 
        base_spacing: f64,
        volatility_multiplier: f64,
    },
    
    /// Trend-following grid that adjusts to market direction
    TrendFollowing {
        base_spacing: f64,
        trend_sensitivity: f64,
    },
    
    /// Support/resistance level based grids
    SupportResistance {
        lookback_periods: usize,
        strength_threshold: f64,
    },
}

/// Grid optimization engine
pub struct GridOptimizer {
    strategies: Vec<GridStrategy>,
}

impl GridOptimizer {
    pub fn new() -> Self {
        Self {
            strategies: vec![
                GridStrategy::Uniform { spacing: 0.01 },
                GridStrategy::Fibonacci { base_spacing: 0.01 },
                GridStrategy::VolatilityAdjusted { 
                    base_spacing: 0.01, 
                    volatility_multiplier: 1.5,
                },
                GridStrategy::TrendFollowing {
                    base_spacing: 0.01,
                    trend_sensitivity: 0.5,
                },
                GridStrategy::SupportResistance {
                    lookback_periods: 50,
                    strength_threshold: 0.7,
                },
            ]
        }
    }

    /// Generate optimized grid levels for a trading pair
    pub fn optimize_grid_strategy(
        &self,
        _trading_pair: &str,
        historical_data: &[f64],  // Price data
        volatility: f64,
    ) -> GridStrategy {
        let mut best_strategy = &self.strategies[0];
        let mut best_score = 0.0;

        for strategy in &self.strategies {
            let score = self.evaluate_grid_strategy(strategy, historical_data, volatility);
            if score > best_score {
                best_score = score;
                best_strategy = strategy;
            }
        }

        best_strategy.clone()
    }

    /// Evaluate a grid strategy against historical data
    fn evaluate_grid_strategy(
        &self,
        strategy: &GridStrategy,
        historical_data: &[f64],
        volatility: f64,
    ) -> f64 {
        match strategy {
            GridStrategy::Uniform { spacing } => {
                self.evaluate_uniform_grid(*spacing, historical_data)
            }
            GridStrategy::Fibonacci { base_spacing } => {
                self.evaluate_fibonacci_grid(*base_spacing, historical_data)
            }
            GridStrategy::VolatilityAdjusted { base_spacing, volatility_multiplier } => {
                let adjusted_spacing = base_spacing * (1.0 + volatility * volatility_multiplier);
                self.evaluate_uniform_grid(adjusted_spacing, historical_data)
            }
            GridStrategy::TrendFollowing { base_spacing, trend_sensitivity } => {
                self.evaluate_trend_following_grid(*base_spacing, *trend_sensitivity, historical_data)
            }
            GridStrategy::SupportResistance { lookback_periods, strength_threshold } => {
                self.evaluate_sr_grid(*lookback_periods, *strength_threshold, historical_data)
            }
        }
    }

    fn evaluate_uniform_grid(&self, spacing: f64, data: &[f64]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let current_price = data[data.len() - 1];
        let mut grid_hits = 0;
        let mut total_profit = 0.0;

        // Simple grid evaluation - count profitable grid crossings
        for window in data.windows(2) {
            let prev_price = window[0];
            let curr_price = window[1];

            // Check for grid level crossings
            let prev_level = (prev_price / (current_price * spacing)).floor();
            let curr_level = (curr_price / (current_price * spacing)).floor();

            if prev_level != curr_level {
                grid_hits += 1;
                total_profit += (curr_price - prev_price).abs() * spacing;
            }
        }

        if grid_hits > 0 {
            total_profit / grid_hits as f64
        } else {
            0.0
        }
    }

    fn evaluate_fibonacci_grid(&self, base_spacing: f64, data: &[f64]) -> f64 {
        // Generate Fibonacci ratios for grid spacing
        let fib_ratios = vec![0.236, 0.382, 0.618, 1.0, 1.618, 2.618];
        let mut total_score = 0.0;

        for ratio in fib_ratios {
            let spacing = base_spacing * ratio;
            total_score += self.evaluate_uniform_grid(spacing, data);
        }

        total_score / 6.0  // Average score
    }

    fn evaluate_trend_following_grid(
        &self,
        base_spacing: f64,
        trend_sensitivity: f64,
        data: &[f64],
    ) -> f64 {
        if data.len() < 20 {
            return 0.0;
        }

        // Calculate trend using moving averages
        let short_ma = self.calculate_moving_average(data, 5);
        let long_ma = self.calculate_moving_average(data, 20);

        let trend_strength = (short_ma - long_ma).abs() / long_ma;
        let adjusted_spacing = base_spacing * (1.0 + trend_strength * trend_sensitivity);

        self.evaluate_uniform_grid(adjusted_spacing, data)
    }

    fn evaluate_sr_grid(
        &self,
        lookback_periods: usize,
        strength_threshold: f64,
        data: &[f64],
    ) -> f64 {
        if data.len() < lookback_periods {
            return 0.0;
        }

        // Identify support and resistance levels
        let sr_levels = self.find_support_resistance_levels(data, lookback_periods, strength_threshold);
        
        // Evaluate grid based on SR levels
        let mut total_score = 0.0;
        for level in sr_levels {
            // Score based on how well the level acts as support/resistance
            total_score += self.evaluate_sr_level_strength(level, data);
        }

        total_score
    }

    fn calculate_moving_average(&self, data: &[f64], period: usize) -> f64 {
        if data.len() < period {
            return data.iter().sum::<f64>() / data.len() as f64;
        }

        let recent_data = &data[data.len() - period..];
        recent_data.iter().sum::<f64>() / period as f64
    }

    fn find_support_resistance_levels(
        &self,
        data: &[f64],
        lookback: usize,
        threshold: f64,
    ) -> Vec<f64> {
        let mut levels = Vec::new();
        
        if data.len() < lookback * 2 {
            return levels;
        }

        // Find local maxima and minima
        for i in lookback..data.len() - lookback {
            let current = data[i];
            let mut is_peak = true;
            let mut is_trough = true;

            // Check if current point is a local maximum or minimum
            for j in i - lookback..i + lookback {
                if j != i {
                    if data[j] >= current {
                        is_peak = false;
                    }
                    if data[j] <= current {
                        is_trough = false;
                    }
                }
            }

            if is_peak || is_trough {
                levels.push(current);
            }
        }

        // Filter levels by strength
        levels.retain(|&level| {
            let touches = data.iter()
                .filter(|&&price| (price - level).abs() / level < threshold)
                .count();
            touches >= 3  // Require at least 3 touches to be significant
        });

        levels
    }

    fn evaluate_sr_level_strength(&self, level: f64, data: &[f64]) -> f64 {
        let touches = data.iter()
            .filter(|&&price| (price - level).abs() / level < 0.01)  // Within 1%
            .count();

        touches as f64 * 0.1  // Simple scoring based on touches
    }
}

impl Default for GridOptimizer {
    fn default() -> Self {
        Self::new()
    }
}