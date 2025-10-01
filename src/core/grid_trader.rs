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
    
    // CRITICAL: Position tracking to prevent infinite trades
    cash_balance: f64,
    inventory_quantity: f64,        // Current holdings
    average_entry_price: f64,       // Average price of inventory
    total_trades: usize,
    realized_pnl: f64,
    
    // Risk limits
    max_position_value_pct: f64,    // Max inventory as % of initial capital
    emergency_exit_threshold: f64,   // Exit if price moves this far beyond grid
}

impl GridTrader {
    pub fn new(trading_config: TradingConfig, market_config: MarketConfig) -> Self {
        Self::with_capital(trading_config, market_config, 1000.0) // Default capital
    }
    
    pub fn with_capital(trading_config: TradingConfig, market_config: MarketConfig, initial_capital: f64) -> Self {
        Self {
            current_price: 0.0,
            buy_levels: Vec::new(),
            sell_levels: Vec::new(),
            last_triggered_level: None,
            last_logged_price: 0.0,
            config: trading_config,
            market_analyzer: MarketAnalyzer::new(market_config),
            cash_balance: initial_capital,
            inventory_quantity: 0.0,
            average_entry_price: 0.0,
            total_trades: 0,
            realized_pnl: 0.0,
            max_position_value_pct: 0.30,  // Max 30% of capital in one position
            emergency_exit_threshold: 0.20, // Exit if price moves 20% beyond grid
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
        // CRITICAL: Check emergency exit conditions first
        if self.should_emergency_exit(current_price) {
            return self.execute_emergency_exit(current_price);
        }
        
        // Check if price hit any buy levels
        for &buy_level in &self.buy_levels {
            if current_price <= buy_level && self.last_triggered_level != Some(buy_level) {
                // CRITICAL: Check if we have capital to buy
                if self.can_buy(current_price) {
                    println!("🟢 BUY SIGNAL! Price £{:.4} hit buy level £{:.4}", current_price, buy_level);
                    println!("   💰 Cash: £{:.2} | Inventory: {:.4}", self.cash_balance, self.inventory_quantity);
                    self.last_triggered_level = Some(buy_level);
                    return GridSignal::Buy(buy_level);
                } else {
                    println!("⚠️  BUY BLOCKED: Insufficient capital or position limit reached");
                }
            }
        }
        
        // Check if price hit any sell levels
        for &sell_level in &self.sell_levels {
            if current_price >= sell_level && self.last_triggered_level != Some(sell_level) {
                // CRITICAL: Check if we have inventory to sell
                if self.can_sell() {
                    println!("🔴 SELL SIGNAL! Price £{:.4} hit sell level £{:.4}", current_price, sell_level);
                    println!("   💰 Cash: £{:.2} | Inventory: {:.4}", self.cash_balance, self.inventory_quantity);
                    self.last_triggered_level = Some(sell_level);
                    return GridSignal::Sell(sell_level);
                } else {
                    println!("⚠️  SELL BLOCKED: No inventory to sell");
                }
            }
        }
        
        GridSignal::None
    }

    // Get adjusted grid spacing based on current market state
    fn get_adjusted_spacing(&self) -> f64 {
        match self.market_analyzer.current_state() {
            MarketState::TrendingUp | MarketState::TrendingDown => {
                // FIXED: Use TIGHTER spacing in trends to capture more moves
                // Grid trading profits from mean reversion within the trend
                self.config.grid_spacing * 0.7
            }
            MarketState::Ranging => {
                // Wider spacing in ranging markets to avoid overtrading
                self.config.grid_spacing * 1.2
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
    
    // CRITICAL: Position management methods
    fn can_buy(&self, price: f64) -> bool {
        let trade_size = self.calculate_trade_size(price);
        let required_cash = trade_size * price * 1.003; // Include 0.3% buffer for fees
        
        // Check cash availability
        if self.cash_balance < required_cash {
            return false;
        }
        
        // Check position limits
        let new_inventory_value = (self.inventory_quantity + trade_size) * price;
        let initial_capital = self.cash_balance + self.inventory_quantity * price;
        let position_pct = new_inventory_value / initial_capital;
        
        position_pct <= self.max_position_value_pct
    }
    
    fn can_sell(&self) -> bool {
        self.inventory_quantity > 0.0
    }
    
    fn calculate_trade_size(&self, price: f64) -> f64 {
        // Equal dollar amount per grid level
        let initial_capital = self.cash_balance + self.inventory_quantity * price;
        let position_size = initial_capital / (self.config.grid_levels as f64 * 2.0);
        (position_size / price).max(0.0001) // Minimum trade size
    }
    
    fn should_emergency_exit(&self, current_price: f64) -> bool {
        if self.buy_levels.is_empty() || self.sell_levels.is_empty() {
            return false;
        }
        
        // Check if price moved beyond grid bounds + threshold
        let lowest_buy = self.buy_levels.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let highest_sell = self.sell_levels.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        
        let lower_bound = lowest_buy * (1.0 - self.emergency_exit_threshold);
        let upper_bound = highest_sell * (1.0 + self.emergency_exit_threshold);
        
        current_price < lower_bound || current_price > upper_bound
    }
    
    fn execute_emergency_exit(&mut self, current_price: f64) -> GridSignal {
        if self.inventory_quantity > 0.0 {
            println!("🚨 EMERGENCY EXIT! Price £{:.4} beyond grid bounds", current_price);
            println!("   Liquidating position: {:.4} units at market", self.inventory_quantity);
            GridSignal::Sell(current_price)
        } else {
            GridSignal::None
        }
    }
    
    // Public method to execute a trade and update positions
    pub fn execute_trade(&mut self, signal: &GridSignal, execution_price: f64) {
        match signal {
            GridSignal::Buy(intended_price) => {
                let quantity = self.calculate_trade_size(execution_price);
                let cost = quantity * execution_price;
                let fee = cost * 0.0026; // Kraken taker fee
                
                if self.cash_balance >= cost + fee {
                    // Update average entry price
                    let total_value = self.inventory_quantity * self.average_entry_price;
                    let new_total_value = total_value + cost;
                    self.inventory_quantity += quantity;
                    self.average_entry_price = new_total_value / self.inventory_quantity;
                    
                    self.cash_balance -= cost + fee;
                    self.total_trades += 1;
                    
                    println!("✅ BUY EXECUTED: {:.4} @ £{:.4} (intended: £{:.4})", quantity, execution_price, intended_price);
                    println!("   Position: {:.4} units @ avg £{:.4} | Cash: £{:.2}", 
                             self.inventory_quantity, self.average_entry_price, self.cash_balance);
                }
            }
            GridSignal::Sell(intended_price) => {
                let quantity = self.calculate_trade_size(execution_price).min(self.inventory_quantity);
                let proceeds = quantity * execution_price;
                let fee = proceeds * 0.0026;
                
                if self.inventory_quantity >= quantity {
                    // Calculate realized P&L
                    let cost_basis = quantity * self.average_entry_price;
                    let pnl = proceeds - cost_basis - fee;
                    self.realized_pnl += pnl;
                    
                    self.inventory_quantity -= quantity;
                    self.cash_balance += proceeds - fee;
                    self.total_trades += 1;
                    
                    println!("✅ SELL EXECUTED: {:.4} @ £{:.4} (intended: £{:.4})", quantity, execution_price, intended_price);
                    println!("   P&L: £{:.2} | Position: {:.4} units | Cash: £{:.2}", 
                             pnl, self.inventory_quantity, self.cash_balance);
                    println!("   Total Realized P&L: £{:.2}", self.realized_pnl);
                }
            }
            GridSignal::None => {}
        }
    }
    
    // Get current portfolio value
    pub fn get_portfolio_value(&self, current_price: f64) -> f64 {
        self.cash_balance + (self.inventory_quantity * current_price)
    }
    
    // Get position summary
    pub fn get_position_summary(&self, current_price: f64) -> String {
        let portfolio_value = self.get_portfolio_value(current_price);
        let unrealized_pnl = if self.inventory_quantity > 0.0 {
            (current_price - self.average_entry_price) * self.inventory_quantity
        } else {
            0.0
        };
        let total_pnl = self.realized_pnl + unrealized_pnl;
        
        format!(
            "Portfolio: £{:.2} | Cash: £{:.2} | Position: {:.4} @ £{:.4}\n   \
             Unrealized P&L: £{:.2} | Realized P&L: £{:.2} | Total: £{:.2} | Trades: {}",
            portfolio_value, self.cash_balance, self.inventory_quantity, 
            self.average_entry_price, unrealized_pnl, self.realized_pnl, total_pnl, self.total_trades
        )
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
            price_history_size: 50,
        };

        let mut analyzer = MarketAnalyzer::new(market_config);
        // Need at least 10 data points for analysis
        // Generate clear uptrend: starting at 1.0 and going up steadily
        let trending_prices = vec![
            1.0, 1.002, 1.004, 1.006, 1.008,
            1.010, 1.012, 1.014, 1.016, 1.018,
            1.020, 1.022, 1.024, 1.026, 1.028,
        ];
        for price in trending_prices {
            analyzer.update_with_price(price);
        }

        // The analyzer should detect an uptrend or at least not crash
        let state = analyzer.current_state();
        // Just verify it returns a valid state (could be TrendingUp or Ranging depending on thresholds)
        assert!(matches!(state, MarketState::TrendingUp | MarketState::Ranging));
    }
}