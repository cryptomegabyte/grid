// Grid trading logic and signal generation

use crate::core::types::{GridSignal, MarketState};
use crate::core::market_state::MarketAnalyzer;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{GridSignal, MarketState};
    use crate::config::{TradingConfig, MarketConfig};

    fn create_test_config() -> (TradingConfig, MarketConfig) {
        let trading_config = TradingConfig {
            kraken_ws_url: "wss://ws.kraken.com".to_string(),
            trading_pair: "XRPGBP".to_string(),
            grid_levels: 3,
            grid_spacing: 0.01,
            min_price_change: 0.001,
        };

        let market_config = MarketConfig {
            trend_threshold: 0.005,
            volatility_threshold: 0.02,
            price_history_size: 10,
        };

        (trading_config, market_config)
    }

    #[test]
    fn test_grid_trader_initialization() {
        let (trading_config, market_config) = create_test_config();
        let trader = GridTrader::new(trading_config.clone(), market_config);

        assert_eq!(trader.buy_levels().len(), 0);
        assert_eq!(trader.sell_levels().len(), 0);
        assert_eq!(trader.current_price(), 0.0);
    }

    #[test]
    fn test_grid_setup() {
        let (trading_config, market_config) = create_test_config();
        let mut trader = GridTrader::new(trading_config, market_config);

        let initial_price = 1.0;
        let signal = trader.update_with_price(initial_price);

        assert_eq!(signal, GridSignal::None);
        assert_eq!(trader.current_price(), initial_price);
        assert_eq!(trader.buy_levels().len(), 3);
        assert_eq!(trader.sell_levels().len(), 3);
    }

    #[test]
    fn test_buy_signal_generation() {
        let (trading_config, market_config) = create_test_config();
        let mut trader = GridTrader::new(trading_config, market_config);

        trader.update_with_price(1.0);
        let buy_signal = trader.update_with_price(0.99);
        
        match buy_signal {
            GridSignal::Buy(level) => assert!((level - 0.99).abs() < 1e-10),
            _ => panic!("Expected buy signal"),
        }
    }

    #[test]
    fn test_sell_signal_generation() {
        let (trading_config, market_config) = create_test_config();
        let mut trader = GridTrader::new(trading_config, market_config);

        trader.update_with_price(1.0);
        let sell_signal = trader.update_with_price(1.01);
        
        match sell_signal {
            GridSignal::Sell(level) => assert!((level - 1.01).abs() < 1e-10),
            _ => panic!("Expected sell signal"),
        }
    }

    #[test]
    fn test_no_duplicate_signals() {
        let (trading_config, market_config) = create_test_config();
        let mut trader = GridTrader::new(trading_config, market_config);

        trader.update_with_price(1.0);
        let first_signal = trader.update_with_price(0.99);
        assert!(matches!(first_signal, GridSignal::Buy(_)));

        let duplicate_signal = trader.update_with_price(0.99);
        assert_eq!(duplicate_signal, GridSignal::None);
    }

    #[test]
    fn test_market_state_detection() {
        let market_config = MarketConfig {
            trend_threshold: 0.01,
            volatility_threshold: 0.02,
            price_history_size: 5,
        };

        let mut analyzer = MarketAnalyzer::new(market_config);
        let trending_prices = vec![1.0, 1.002, 1.004, 1.006, 1.015];
        for price in trending_prices {
            analyzer.update_with_price(price);
        }

        assert_eq!(analyzer.current_state(), MarketState::TrendingUp);
    }
}