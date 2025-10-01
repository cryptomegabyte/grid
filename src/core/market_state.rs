// Enhanced market state detection and analysis

use crate::core::types::MarketState;
use crate::config::MarketConfig;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct MarketAnalyzer {
    price_history: VecDeque<f64>,
    volume_history: VecDeque<f64>,
    current_state: MarketState,
    config: MarketConfig,
    
    // Technical indicators
    sma_short: f64,
    sma_long: f64,
    ema_fast: f64,
    ema_slow: f64,
    rsi: f64,
    bollinger_upper: f64,
    bollinger_lower: f64,
    
    // Advanced metrics
    volume_weighted_price: f64,
    price_momentum: f64,
    volatility_regime: VolatilityRegime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VolatilityRegime {
    Low,      // Low volatility period
    Normal,   // Normal volatility
    High,     // High volatility period
    Extreme,  // Extreme volatility (potential crisis)
}

impl Default for MarketAnalyzer {
    fn default() -> Self {
        Self::new(MarketConfig::default())
    }
}

impl MarketAnalyzer {
    pub fn new(config: MarketConfig) -> Self {
        Self {
            price_history: VecDeque::with_capacity(config.price_history_size),
            volume_history: VecDeque::with_capacity(config.price_history_size),
            current_state: MarketState::Ranging,
            config,
            
            // Initialize technical indicators
            sma_short: 0.0,
            sma_long: 0.0,
            ema_fast: 0.0,
            ema_slow: 0.0,
            rsi: 50.0,
            bollinger_upper: 0.0,
            bollinger_lower: 0.0,
            
            // Initialize advanced metrics
            volume_weighted_price: 0.0,
            price_momentum: 0.0,
            volatility_regime: VolatilityRegime::Normal,
        }
    }

    pub fn current_state(&self) -> MarketState {
        self.current_state
    }

    pub fn update_with_price(&mut self, new_price: f64) -> Option<MarketState> {
        self.update_with_price_and_volume(new_price, 0.0)
    }

    pub fn update_with_price_and_volume(&mut self, new_price: f64, volume: f64) -> Option<MarketState> {
        // Add new data to history
        self.price_history.push_back(new_price);
        self.volume_history.push_back(volume);
        
        // Keep only configured number of data points
        while self.price_history.len() > self.config.price_history_size {
            self.price_history.pop_front();
        }
        while self.volume_history.len() > self.config.price_history_size {
            self.volume_history.pop_front();
        }
        
        // Need sufficient data for analysis
        if self.price_history.len() < 10 {
            return None;
        }
        
        // Update technical indicators
        self.update_technical_indicators();
        
        let old_state = self.current_state;
        let new_state = self.detect_advanced_market_state();
        
        if old_state != new_state {
            println!("🔄 Market state changed: {:?} → {:?} (RSI: {:.1}, Vol Regime: {:?})", 
                     old_state, new_state, self.rsi, self.volatility_regime);
            self.current_state = new_state;
            Some(new_state)
        } else {
            None
        }
    }

    fn update_technical_indicators(&mut self) {
        let prices: Vec<f64> = self.price_history.iter().cloned().collect();
        let volumes: Vec<f64> = self.volume_history.iter().cloned().collect();
        
        if prices.len() < 10 {
            return;
        }
        
        // Update moving averages
        self.sma_short = self.calculate_sma(&prices, 5);
        self.sma_long = self.calculate_sma(&prices, 20);
        
        // Update exponential moving averages
        self.ema_fast = self.calculate_ema(&prices, 12);
        self.ema_slow = self.calculate_ema(&prices, 26);
        
        // Update RSI
        self.rsi = self.calculate_rsi(&prices, 14);
        
        // Update Bollinger Bands
        let (upper, lower) = self.calculate_bollinger_bands(&prices, 20, 2.0);
        self.bollinger_upper = upper;
        self.bollinger_lower = lower;
        
        // Update volume-weighted price
        self.volume_weighted_price = self.calculate_vwap(&prices, &volumes);
        
        // Update momentum
        self.price_momentum = self.calculate_momentum(&prices, 10);
        
        // Update volatility regime
        self.volatility_regime = self.detect_volatility_regime(&prices);
    }

    fn calculate_sma(&self, prices: &[f64], period: usize) -> f64 {
        if prices.len() < period {
            return prices.iter().sum::<f64>() / prices.len() as f64;
        }
        
        let start = prices.len() - period;
        prices[start..].iter().sum::<f64>() / period as f64
    }

    fn calculate_ema(&self, prices: &[f64], period: usize) -> f64 {
        if prices.is_empty() {
            return 0.0;
        }
        
        let alpha = 2.0 / (period as f64 + 1.0);
        let mut ema = prices[0];
        
        for &price in &prices[1..] {
            ema = alpha * price + (1.0 - alpha) * ema;
        }
        
        ema
    }

    fn calculate_rsi(&self, prices: &[f64], period: usize) -> f64 {
        if prices.len() < period + 1 {
            return 50.0;
        }
        
        let mut gains = 0.0;
        let mut losses = 0.0;
        
        for i in 1..=period {
            let idx = prices.len() - period - 1 + i;
            let change = prices[idx] - prices[idx - 1];
            
            if change >= 0.0 {
                gains += change;
            } else {
                losses += change.abs();
            }
        }
        
        let avg_gain = gains / period as f64;
        let avg_loss = losses / period as f64;
        
        if avg_loss == 0.0 {
            return 100.0;
        }
        
        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }

    fn calculate_bollinger_bands(&self, prices: &[f64], period: usize, std_dev: f64) -> (f64, f64) {
        let sma = self.calculate_sma(prices, period);
        
        if prices.len() < period {
            return (sma, sma);
        }
        
        let start = prices.len() - period;
        let variance = prices[start..].iter()
            .map(|&p| (p - sma).powi(2))
            .sum::<f64>() / period as f64;
        
        let std = variance.sqrt();
        
        (sma + std_dev * std, sma - std_dev * std)
    }

    fn calculate_vwap(&self, prices: &[f64], volumes: &[f64]) -> f64 {
        if prices.len() != volumes.len() || prices.is_empty() {
            return prices.last().cloned().unwrap_or(0.0);
        }
        
        let mut total_volume = 0.0;
        let mut total_value = 0.0;
        
        for (price, volume) in prices.iter().zip(volumes.iter()) {
            total_value += price * volume;
            total_volume += volume;
        }
        
        if total_volume > 0.0 {
            total_value / total_volume
        } else {
            prices.last().cloned().unwrap_or(0.0)
        }
    }

    fn calculate_momentum(&self, prices: &[f64], period: usize) -> f64 {
        if prices.len() <= period {
            return 0.0;
        }
        
        let current = *prices.last().unwrap();
        let past = prices[prices.len() - period - 1];
        
        (current - past) / past
    }

    fn detect_volatility_regime(&self, prices: &[f64]) -> VolatilityRegime {
        if prices.len() < 20 {
            return VolatilityRegime::Normal;
        }
        
        // Calculate 20-period rolling volatility
        let returns: Vec<f64> = prices.windows(2)
            .map(|w| (w[1] - w[0]) / w[0])
            .collect();
        
        let volatility = if returns.len() >= 20 {
            let recent_returns = &returns[returns.len() - 20..];
            let mean = recent_returns.iter().sum::<f64>() / recent_returns.len() as f64;
            let variance = recent_returns.iter()
                .map(|&r| (r - mean).powi(2))
                .sum::<f64>() / recent_returns.len() as f64;
            variance.sqrt()
        } else {
            0.01 // Default low volatility
        };
        
        // Volatility thresholds (annualized)
        let annualized_vol = volatility * (252.0_f64).sqrt(); // Assuming daily data
        
        match annualized_vol {
            vol if vol < 0.10 => VolatilityRegime::Low,      // < 10%
            vol if vol < 0.20 => VolatilityRegime::Normal,   // 10-20%
            vol if vol < 0.40 => VolatilityRegime::High,     // 20-40%
            _ => VolatilityRegime::Extreme,                   // > 40%
        }
    }

    fn detect_advanced_market_state(&self) -> MarketState {
        let current_price = *self.price_history.back().unwrap();
        
        // Multi-factor state detection
        let mut trend_score = 0.0;
        let mut ranging_score = 0.0;
        
        // 1. Moving average trends
        if self.sma_short > self.sma_long {
            trend_score += 1.0;
        } else if self.sma_short < self.sma_long {
            trend_score -= 1.0;
        }
        
        // 2. EMA crossover
        if self.ema_fast > self.ema_slow {
            trend_score += 0.5;
        } else {
            trend_score -= 0.5;
        }
        
        // 3. Price relative to Bollinger Bands
        if current_price > self.bollinger_upper {
            trend_score += 0.5; // Breakout upward
        } else if current_price < self.bollinger_lower {
            trend_score -= 0.5; // Breakout downward
        } else {
            ranging_score += 1.0; // Within bands
        }
        
        // 4. RSI levels
        if self.rsi > 70.0 {
            trend_score += 0.3; // Overbought but trending
        } else if self.rsi < 30.0 {
            trend_score -= 0.3; // Oversold but trending
        } else if self.rsi > 40.0 && self.rsi < 60.0 {
            ranging_score += 0.5; // Neutral RSI suggests ranging
        }
        
        // 5. Momentum
        if self.price_momentum.abs() > 0.02 { // 2% momentum threshold
            if self.price_momentum > 0.0 {
                trend_score += 0.5;
            } else {
                trend_score -= 0.5;
            }
        } else {
            ranging_score += 0.5;
        }
        
        // 6. Volatility regime influence
        match self.volatility_regime {
            VolatilityRegime::Low => ranging_score += 0.3,
            VolatilityRegime::Extreme => trend_score *= 1.2, // Amplify trends in extreme vol
            _ => {}
        }
        
        // Final state determination
        // Consider ranging score in the decision
        if ranging_score > 2.0 {
            MarketState::Ranging
        } else if trend_score > 1.5 {
            MarketState::TrendingUp
        } else if trend_score < -1.5 {
            MarketState::TrendingDown
        } else {
            MarketState::Ranging
        }
    }

    pub fn get_price_change_info(&self) -> Option<(f64, f64)> {
        if self.price_history.len() < 2 {
            return None;
        }

        let first_price = self.price_history.front().unwrap();
        let last_price = self.price_history.back().unwrap();
        let price_change_pct = (last_price - first_price) / first_price;
        
        let prices: Vec<f64> = self.price_history.iter().cloned().collect();
        let avg_price: f64 = prices.iter().sum::<f64>() / prices.len() as f64;
        let variance: f64 = prices.iter()
            .map(|&p| (p - avg_price).powi(2))
            .sum::<f64>() / prices.len() as f64;
        let volatility = variance.sqrt() / avg_price;

        Some((price_change_pct * 100.0, volatility * 100.0))
    }

    // New methods for enhanced market analysis
    pub fn get_technical_indicators(&self) -> TechnicalIndicators {
        TechnicalIndicators {
            sma_short: self.sma_short,
            sma_long: self.sma_long,
            ema_fast: self.ema_fast,
            ema_slow: self.ema_slow,
            rsi: self.rsi,
            bollinger_upper: self.bollinger_upper,
            bollinger_lower: self.bollinger_lower,
            volume_weighted_price: self.volume_weighted_price,
            price_momentum: self.price_momentum,
            volatility_regime: self.volatility_regime.clone(),
        }
    }

    pub fn get_trend_strength(&self) -> f64 {
        // Calculate trend strength based on multiple indicators
        let mut strength = 0.0;
        
        // MA alignment
        if (self.sma_short - self.sma_long).abs() / self.sma_long > 0.01 {
            strength += 0.3;
        }
        
        // EMA divergence
        if (self.ema_fast - self.ema_slow).abs() / self.ema_slow > 0.005 {
            strength += 0.2;
        }
        
        // Momentum
        strength += self.price_momentum.abs().min(0.1) * 5.0; // Scale momentum to 0-0.5
        
        // RSI extremes
        if self.rsi > 70.0 || self.rsi < 30.0 {
            strength += 0.2;
        }
        
        strength.min(1.0) // Cap at 1.0
    }

    pub fn is_oversold(&self) -> bool {
        self.rsi < 30.0
    }

    pub fn is_overbought(&self) -> bool {
        self.rsi > 70.0
    }

    pub fn get_support_resistance_levels(&self) -> (Option<f64>, Option<f64>) {
        if self.price_history.len() < 10 {
            return (None, None);
        }
        
        let prices: Vec<f64> = self.price_history.iter().cloned().collect();
        
        // Simple support/resistance based on recent highs and lows
        let recent_high = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let recent_low = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        (Some(recent_low), Some(recent_high))
    }
}

#[derive(Debug, Clone)]
pub struct TechnicalIndicators {
    pub sma_short: f64,
    pub sma_long: f64,
    pub ema_fast: f64,
    pub ema_slow: f64,
    pub rsi: f64,
    pub bollinger_upper: f64,
    pub bollinger_lower: f64,
    pub volume_weighted_price: f64,
    pub price_momentum: f64,
    pub volatility_regime: VolatilityRegime,
}