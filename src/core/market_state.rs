// Market state detection and analysis

use crate::core::types::MarketState;
use crate::config::MarketConfig;

pub struct MarketAnalyzer {
    price_history: Vec<f64>,
    current_state: MarketState,
    config: MarketConfig,
}

impl Default for MarketAnalyzer {
    fn default() -> Self {
        Self::new(MarketConfig::default())
    }
}

impl MarketAnalyzer {
    pub fn new(config: MarketConfig) -> Self {
        Self {
            price_history: Vec::new(),
            current_state: MarketState::Ranging,
            config,
        }
    }

    pub fn current_state(&self) -> MarketState {
        self.current_state
    }

    pub fn update_with_price(&mut self, new_price: f64) -> Option<MarketState> {
        // Add new price to history
        self.price_history.push(new_price);
        
        // Keep only configured number of prices for analysis
        if self.price_history.len() > self.config.price_history_size {
            self.price_history.remove(0);
        }
        
        // Need at least 5 prices to detect state
        if self.price_history.len() < 5 {
            return None;
        }
        
        let old_state = self.current_state;
        let new_state = self.detect_market_state();
        
        if old_state != new_state {
            println!("🔄 Market state changed: {:?} → {:?}", old_state, new_state);
            self.current_state = new_state;
            Some(new_state)
        } else {
            None
        }
    }

    fn detect_market_state(&self) -> MarketState {
        let first_price = self.price_history[0];
        let last_price = *self.price_history.last().unwrap();
        let price_change_pct = (last_price - first_price) / first_price;
        
        // Calculate volatility (simple standard deviation)
        let avg_price: f64 = self.price_history.iter().sum::<f64>() / self.price_history.len() as f64;
        let variance: f64 = self.price_history.iter()
            .map(|&p| (p - avg_price).powi(2))
            .sum::<f64>() / self.price_history.len() as f64;
        let volatility = variance.sqrt() / avg_price; // Coefficient of variation
        
        // State detection logic using config thresholds
        if price_change_pct > self.config.trend_threshold && volatility < self.config.volatility_threshold {
            MarketState::TrendingUp
        } else if price_change_pct < -self.config.trend_threshold && volatility < self.config.volatility_threshold {
            MarketState::TrendingDown
        } else {
            MarketState::Ranging  // High volatility or small moves
        }
    }

    pub fn get_price_change_info(&self) -> Option<(f64, f64)> {
        if self.price_history.len() < 2 {
            return None;
        }

        let first_price = self.price_history[0];
        let last_price = *self.price_history.last().unwrap();
        let price_change_pct = (last_price - first_price) / first_price;
        
        let avg_price: f64 = self.price_history.iter().sum::<f64>() / self.price_history.len() as f64;
        let variance: f64 = self.price_history.iter()
            .map(|&p| (p - avg_price).powi(2))
            .sum::<f64>() / self.price_history.len() as f64;
        let volatility = variance.sqrt() / avg_price;

        Some((price_change_pct * 100.0, volatility * 100.0))
    }
}