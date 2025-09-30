// Grid trading logic and signal generation

use crate::types::{GridSignal, MarketState};
use crate::market_state::MarketAnalyzer;
use crate::config::{TradingConfig, MarketConfig};

pub struct GridTrader {
    current_price: f64,
    buy_levels: Vec<f64>,
    sell_levels: Vec<f64>,
    last_triggered_level: Option<f64>,
    last_logged_price: f64,
    config: TradingConfig,
    market_analyzer: MarketAnalyzer,
}

impl GridTrader {
    pub fn new(trading_config: TradingConfig, market_config: MarketConfig) -> Self {
        Self {
            current_price: 0.0,
            buy_levels: Vec::new(),
            sell_levels: Vec::new(),
            last_triggered_level: None,
            last_logged_price: 0.0,
            config: trading_config,
            market_analyzer: MarketAnalyzer::new(market_config),
        }
    }

    pub fn update_with_price(&mut self, new_price: f64) -> GridSignal {
        // Update market state analysis
        if let Some(_new_state) = self.market_analyzer.update_with_price(new_price) {
            // Market state changed - rebuild grid if we have levels
            if !self.buy_levels.is_empty() {
                self.setup_grid(new_price);
            }
        }
        
        // Initialize grid if this is the first price update
        if self.buy_levels.is_empty() {
            self.setup_grid(new_price);
        }
        
        // Check for trading signals
        let signal = self.check_grid_signals(new_price);
        self.current_price = new_price;
        signal
    }

    pub fn should_log_price(&self, price: f64, min_change: f64) -> bool {
        (price - self.last_logged_price).abs() > min_change
    }

    pub fn update_logged_price(&mut self, price: f64) {
        self.last_logged_price = price;
    }

    fn setup_grid(&mut self, center_price: f64) {
        self.current_price = center_price;
        self.buy_levels.clear();
        self.sell_levels.clear();
        
        // Get adjusted spacing based on market state
        let adjusted_spacing = self.get_adjusted_spacing();
        
        // Create buy levels below current price
        for i in 1..=self.config.grid_levels {
            let buy_level = center_price - (i as f64 * adjusted_spacing);
            self.buy_levels.push(buy_level);
        }
        
        // Create sell levels above current price
        for i in 1..=self.config.grid_levels {
            let sell_level = center_price + (i as f64 * adjusted_spacing);
            self.sell_levels.push(sell_level);
        }
        
        self.log_grid_setup(adjusted_spacing);
    }

    fn log_grid_setup(&self, spacing: f64) {
        println!("🎯 Grid Setup Complete! (State: {:?}, Spacing: £{:.4})", 
                 self.market_analyzer.current_state(), spacing);
        println!("   📉 Buy levels:  {:?}", 
                 self.buy_levels.iter().map(|&x| format!("£{:.4}", x)).collect::<Vec<_>>());
        println!("   📈 Sell levels: {:?}", 
                 self.sell_levels.iter().map(|&x| format!("£{:.4}", x)).collect::<Vec<_>>());
        
        // Log price change info if available
        if let Some((price_change, volatility)) = self.market_analyzer.get_price_change_info() {
            println!("   📊 Price change: {:.2}%, Volatility: {:.2}%", price_change, volatility);
        }
    }
    
    fn check_grid_signals(&mut self, current_price: f64) -> GridSignal {
        // Check if price hit any buy levels
        for &buy_level in &self.buy_levels {
            if current_price <= buy_level && self.last_triggered_level != Some(buy_level) {
                println!("🟢 BUY SIGNAL! Price £{:.4} hit buy level £{:.4}", current_price, buy_level);
                self.last_triggered_level = Some(buy_level);
                return GridSignal::Buy(buy_level);
            }
        }
        
        // Check if price hit any sell levels
        for &sell_level in &self.sell_levels {
            if current_price >= sell_level && self.last_triggered_level != Some(sell_level) {
                println!("🔴 SELL SIGNAL! Price £{:.4} hit sell level £{:.4}", current_price, sell_level);
                self.last_triggered_level = Some(sell_level);
                return GridSignal::Sell(sell_level);
            }
        }
        
        GridSignal::None
    }

    // Get adjusted grid spacing based on current market state
    fn get_adjusted_spacing(&self) -> f64 {
        match self.market_analyzer.current_state() {
            MarketState::TrendingUp | MarketState::TrendingDown => {
                // Wider spacing in trending markets to avoid too many signals
                self.config.grid_spacing * 1.5
            }
            MarketState::Ranging => {
                // Normal spacing in ranging markets
                self.config.grid_spacing
            }
        }
    }

    // Public getter methods for testing
    pub fn current_price(&self) -> f64 {
        self.current_price
    }

    pub fn buy_levels(&self) -> &Vec<f64> {
        &self.buy_levels
    }

    pub fn sell_levels(&self) -> &Vec<f64> {
        &self.sell_levels
    }

    pub fn market_state(&self) -> MarketState {
        self.market_analyzer.current_state()
    }
}