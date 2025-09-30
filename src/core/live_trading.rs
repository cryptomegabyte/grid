// Live Trading Engine with Realistic Simulation
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::time::{sleep, Duration, Instant};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use rand::{thread_rng, Rng};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedStrategy {
    pub trading_pair: String,
    pub grid_levels: u32,
    pub grid_spacing: f64,
    pub expected_return: f64,
    pub total_trades: usize,
    pub win_rate: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub total_fees: f64,
    pub markov_confidence: f64,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LiveStrategy {
    pub pair: String,
    pub config: OptimizedStrategy,
    pub grid_levels: Vec<f64>,
    pub current_position: f64,
    pub available_capital: f64,
    pub active_orders: Vec<SimulatedOrder>,
}

#[derive(Debug, Clone)]
pub struct SimulatedOrder {
    pub id: String,
    pub pair: String,
    pub side: String, // "buy" or "sell"
    pub price: f64,
    pub quantity: f64,
    pub timestamp: DateTime<Utc>,
    pub status: OrderStatus,
}

#[derive(Debug, Clone)]
pub enum OrderStatus {
    Pending,
    Filled,
    PartiallyFilled(f64), // filled quantity
    Cancelled,
    Rejected(String),
}

pub struct LiveTradingEngine {
    strategies: HashMap<String, LiveStrategy>,
    portfolio: PortfolioState,
    total_capital: f64,
    trade_history: Vec<SimulatedTrade>,
    #[allow(dead_code)]
    kraken_client: reqwest::Client,
    current_prices: HashMap<String, PriceData>,
    trade_log_file: String,
    portfolio_log_file: String,
    last_portfolio_update: Instant,
}

#[derive(Debug, Clone)]
pub struct PriceData {
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub volume: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PortfolioState {
    pub cash_balance: f64,
    pub positions: HashMap<String, f64>, // pair -> quantity
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub total_fees_paid: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulatedTrade {
    pub id: String,
    pub pair: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub timestamp: DateTime<Utc>,
    pub execution_delay_ms: u64,
    pub slippage: f64,
}

impl LiveTradingEngine {
    pub fn new(initial_capital: f64) -> Self {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        
        Self {
            strategies: HashMap::new(),
            portfolio: PortfolioState {
                cash_balance: initial_capital,
                positions: HashMap::new(),
                unrealized_pnl: 0.0,
                realized_pnl: 0.0,
                total_fees_paid: 0.0,
            },
            total_capital: initial_capital,
            trade_history: Vec::new(),
            kraken_client: reqwest::Client::new(), // Public API only for simulation
            current_prices: HashMap::new(),
            trade_log_file: format!("trade_log_{}.csv", timestamp),
            portfolio_log_file: format!("portfolio_log_{}.csv", timestamp),
            last_portfolio_update: Instant::now(),
        }
    }

    /// Load all optimized strategies from the optimized_strategies folder
    pub fn load_optimized_strategies<P: AsRef<Path>>(&mut self, strategies_dir: P) -> Result<usize, Box<dyn std::error::Error>> {
        let dir_path = strategies_dir.as_ref();
        info!("🔍 Loading optimized strategies from: {}", dir_path.display());

        if !dir_path.exists() {
            warn!("Strategies directory does not exist: {}", dir_path.display());
            return Ok(0);
        }

        let mut loaded_count = 0;
        let entries = fs::read_dir(dir_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_single_strategy(&path) {
                    Ok(strategy) => {
                        info!("✅ Loaded strategy for {}", strategy.pair);
                        self.strategies.insert(strategy.pair.clone(), strategy);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        error!("❌ Failed to load strategy from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("🎯 Successfully loaded {} optimized strategies", loaded_count);
        Ok(loaded_count)
    }

    /// Load a single strategy file and convert to LiveStrategy
    fn load_single_strategy<P: AsRef<Path>>(&self, file_path: P) -> Result<LiveStrategy, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let optimized: OptimizedStrategy = serde_json::from_str(&content)?;
        
        // Calculate grid levels based on current market price (we'll get this from WebSocket later)
        let estimated_price = 1.0; // Placeholder - will be replaced with live price
        let grid_levels = self.calculate_grid_levels(estimated_price, optimized.grid_spacing, optimized.grid_levels);
        
        // Allocate capital per strategy (equal allocation for now)
        let capital_per_strategy = self.total_capital / 20.0; // Assuming max 20 strategies
        
        Ok(LiveStrategy {
            pair: optimized.trading_pair.clone(),
            config: optimized,
            grid_levels,
            current_position: 0.0,
            available_capital: capital_per_strategy,
            active_orders: Vec::new(),
        })
    }

    /// Calculate grid levels around current price
    fn calculate_grid_levels(&self, current_price: f64, spacing: f64, levels: u32) -> Vec<f64> {
        let mut grid_levels = Vec::new();
        let half_levels = levels / 2;
        
        // Generate buy levels (below current price)
        for i in 1..=half_levels {
            let price = current_price * (1.0 - spacing * i as f64);
            grid_levels.push(price);
        }
        
        // Generate sell levels (above current price)
        for i in 1..=half_levels {
            let price = current_price * (1.0 + spacing * i as f64);
            grid_levels.push(price);
        }
        
        grid_levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        grid_levels
    }

    /// Start the live trading simulation (indefinite)
    pub async fn start_simulation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting live trading simulation with {} strategies", self.strategies.len());
        self.run_trading_loop(None).await
    }

    /// Start the live trading simulation with duration limit
    pub async fn start_simulation_with_duration(&mut self, duration: Duration) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting live trading simulation with {} strategies for {:.1} hours", 
              self.strategies.len(), duration.as_secs_f64() / 3600.0);
        self.run_trading_loop(Some(duration)).await
    }

    /// Internal trading loop with optional duration
    async fn run_trading_loop(&mut self, duration: Option<Duration>) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        // Main trading loop
        loop {
            // Check if we should stop due to duration limit
            if let Some(duration) = duration {
                if start_time.elapsed() >= duration {
                    info!("⏰ Trading session completed after {:.1} hours", duration.as_secs_f64() / 3600.0);
                    break;
                }
            }

            // 1. Get live prices for all pairs
            self.update_live_prices().await?;
            
            // 2. Check for grid triggers
            self.check_grid_triggers().await;
            
            // 3. Process pending orders
            self.process_pending_orders().await;
            
            // 4. Update portfolio state
            self.update_portfolio_state();
            
            // 5. Log performance metrics
            self.log_performance_update();
            
            // 6. Sleep before next iteration
            sleep(Duration::from_millis(100)).await; // 10 updates per second
        }
        
        Ok(())
    }

    async fn update_live_prices(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for pair in self.strategies.keys() {
            match self.fetch_live_price(pair).await {
                Ok(price_data) => {
                    self.current_prices.insert(pair.clone(), price_data);
                }
                Err(e) => {
                    warn!("⚠️  Failed to fetch price for {}: {}", pair, e);
                }
            }
        }
        Ok(())
    }

    async fn fetch_live_price(&self, pair: &str) -> Result<PriceData, Box<dyn std::error::Error>> {
        // For simulation, generate realistic price movements
        // In production, this would fetch from Kraken API
        let mut rng = thread_rng();
        let base_price = match pair {
            "ADAGBP" => 0.4250,
            "LINKGBP" => 8.5000,
            "ETHGBP" => 2100.0,
            "BTCGBP" => 45000.0,
            _ => 1.0000, // Default price
        };
        
        // Simulate price movement (±0.1% random walk)
        let price_change = rng.gen_range(-0.001..0.001);
        let current_price = base_price * (1.0 + price_change);
        
        // Simulate bid-ask spread (2-5 basis points)
        let spread_bps = rng.gen_range(2.0..5.0) / 10000.0;
        let bid = current_price * (1.0 - spread_bps);
        let ask = current_price * (1.0 + spread_bps);
        
        Ok(PriceData {
            bid,
            ask,
            last: current_price,
            volume: rng.gen_range(1000.0..10000.0),
            timestamp: Utc::now(),
        })
    }

    async fn check_grid_triggers(&mut self) {
        let mut orders_to_place = Vec::new();
        
        // First pass: collect orders to place (avoid borrowing conflicts)
        {
            let strategies = &self.strategies;
            let current_prices = &self.current_prices;
            
            for (pair, strategy) in strategies {
                if let Some(price_data) = current_prices.get(pair) {
                    let mut updated_strategy = strategy.clone();
                    self.update_grid_levels(&mut updated_strategy, price_data.last);
                    
                    // Check for buy/sell triggers
                    for &level in &updated_strategy.grid_levels {
                        if level < price_data.last && !self.has_pending_order_at_level(pair, level, "buy") {
                            if price_data.last * 0.995 <= level {
                                orders_to_place.push((pair.clone(), "buy".to_string(), level, strategy.available_capital * 0.05));
                            }
                        } else if level > price_data.last && !self.has_pending_order_at_level(pair, level, "sell") {
                            if price_data.last * 1.005 >= level && strategy.current_position > 0.0 {
                                orders_to_place.push((pair.clone(), "sell".to_string(), level, strategy.current_position * 0.2));
                            }
                        }
                    }
                }
            }
        }
        
        // Second pass: place orders
        for (pair, side, price, quantity) in orders_to_place {
            self.place_simulated_order(&pair, &side, price, quantity).await;
        }
    }

    fn update_grid_levels(&self, strategy: &mut LiveStrategy, current_price: f64) {
        let spacing = strategy.config.grid_spacing;
        let levels = strategy.config.grid_levels;
        
        // Recalculate grid levels around current price
        strategy.grid_levels = self.calculate_grid_levels(current_price, spacing, levels);
    }

    fn has_pending_order_at_level(&self, pair: &str, level: f64, side: &str) -> bool {
        self.strategies.get(pair)
            .map(|s| s.active_orders.iter()
                .any(|order| order.side == side && 
                    (order.price - level).abs() < level * 0.001 && // Within 0.1%
                    matches!(order.status, OrderStatus::Pending)
                ))
            .unwrap_or(false)
    }

    async fn place_simulated_order(&mut self, pair: &str, side: &str, price: f64, quantity: f64) {
        // Apply realistic constraints
        if quantity < 1.0 { // Minimum order size
            debug!("� Order too small: {} units for {}", quantity, pair);
            return;
        }

        let order_id = Uuid::new_v4().to_string();
        let order = SimulatedOrder {
            id: order_id.clone(),
            pair: pair.to_string(),
            side: side.to_string(),
            price,
            quantity,
            timestamp: Utc::now(),
            status: OrderStatus::Pending,
        };

        // Add to strategy's active orders
        if let Some(strategy) = self.strategies.get_mut(pair) {
            strategy.active_orders.push(order.clone());
            info!("📝 Placed {} order: {} {} @ £{:.6} (ID: {})", 
                  side.to_uppercase(), quantity, pair, price, &order_id[..8]);
        }
    }

    async fn process_pending_orders(&mut self) {
        let mut orders_to_process = Vec::new();
        
        // Collect orders that might execute
        for (pair, strategy) in &self.strategies {
            if let Some(price_data) = self.current_prices.get(pair) {
                for order in &strategy.active_orders {
                    if matches!(order.status, OrderStatus::Pending) {
                        if self.should_execute_order(order, price_data) {
                            orders_to_process.push((pair.clone(), order.clone()));
                        }
                    }
                }
            }
        }

        // Process executions
        for (pair, order) in orders_to_process {
            self.execute_simulated_order(&pair, &order).await;
        }
    }

    fn should_execute_order(&self, order: &SimulatedOrder, price_data: &PriceData) -> bool {
        // Simulate realistic execution conditions
        let mut rng = thread_rng();
        
        // Check if price crosses order level
        let crosses_level = match order.side.as_str() {
            "buy" => price_data.ask <= order.price, // Can buy at ask price <= order price
            "sell" => price_data.bid >= order.price, // Can sell at bid price >= order price
            _ => false,
        };

        if !crosses_level {
            return false;
        }

        // Simulate liquidity and execution probability (90% fill rate)
        let fill_probability = 0.9;
        let _execution_delay = Duration::from_millis(rng.gen_range(50..200)); // 50-200ms delay
        
        // For simulation, we'll execute immediately but log the delay
        rng.gen::<f64>() < fill_probability
    }

    async fn execute_simulated_order(&mut self, pair: &str, order: &SimulatedOrder) {
        let mut rng = thread_rng();
        
        // Calculate realistic execution price with slippage
        let price_data = self.current_prices.get(pair).unwrap();
        let slippage_bps = rng.gen_range(1.0..5.0); // 1-5 basis points slippage
        let slippage_factor = slippage_bps / 10000.0;
        
        let execution_price = match order.side.as_str() {
            "buy" => price_data.ask * (1.0 + slippage_factor), // Buy at higher price
            "sell" => price_data.bid * (1.0 - slippage_factor), // Sell at lower price
            _ => order.price,
        };

        // Calculate fees (Kraken taker fee ~0.26%)
        let fee_rate = 0.0026;
        let trade_value = execution_price * order.quantity;
        let fees = trade_value * fee_rate;
        let slippage_cost = (execution_price - order.price).abs() * order.quantity;

        // Create executed trade
        let trade = SimulatedTrade {
            id: order.id.clone(),
            pair: pair.to_string(),
            side: order.side.clone(),
            price: execution_price,
            quantity: order.quantity,
            fee: fees,
            timestamp: Utc::now(),
            execution_delay_ms: rng.gen_range(50..200),
            slippage: slippage_cost,
        };

        // Update portfolio
        self.update_portfolio_from_trade(&trade);
        
        // Update strategy position
        if let Some(strategy) = self.strategies.get_mut(pair) {
            match trade.side.as_str() {
                "buy" => {
                    strategy.current_position += trade.quantity;
                    strategy.available_capital -= trade_value + fees;
                }
                "sell" => {
                    strategy.current_position -= trade.quantity;
                    strategy.available_capital += trade_value - fees;
                }
                _ => {}
            }

            // Update order status
            for active_order in &mut strategy.active_orders {
                if active_order.id == order.id {
                    active_order.status = OrderStatus::Filled;
                    break;
                }
            }
        }

        // Log trade
        self.log_trade(&trade);
        self.trade_history.push(trade.clone());

        info!("✅ EXECUTED: {} {} {} @ £{:.6} | Fee: £{:.2} | Slippage: £{:.2}", 
              trade.side.to_uppercase(), trade.quantity, trade.pair, 
              trade.price, trade.fee, trade.slippage);
    }

    fn update_portfolio_from_trade(&mut self, trade: &SimulatedTrade) {
        let trade_value = trade.price * trade.quantity;
        
        match trade.side.as_str() {
            "buy" => {
                self.portfolio.cash_balance -= trade_value + trade.fee;
                *self.portfolio.positions.entry(trade.pair.clone()).or_insert(0.0) += trade.quantity;
            }
            "sell" => {
                self.portfolio.cash_balance += trade_value - trade.fee;
                *self.portfolio.positions.entry(trade.pair.clone()).or_insert(0.0) -= trade.quantity;
            }
            _ => {}
        }
        
        self.portfolio.total_fees_paid += trade.fee;
    }

    fn update_portfolio_state(&mut self) {
        // Calculate unrealized P&L
        let mut total_unrealized_pnl = 0.0;
        
        for (pair, position) in &self.portfolio.positions {
            if let Some(price_data) = self.current_prices.get(pair) {
                let position_value = *position * price_data.last;
                // This is a simplified calculation - in reality we'd track cost basis
                total_unrealized_pnl += position_value;
            }
        }
        
        self.portfolio.unrealized_pnl = total_unrealized_pnl;
        
        // Log portfolio state every 30 seconds
        if self.last_portfolio_update.elapsed() > Duration::from_secs(30) {
            self.log_portfolio_state();
            self.last_portfolio_update = Instant::now();
        }
    }

    fn log_trade(&self, trade: &SimulatedTrade) {
        // Log to CSV file
        let log_entry = format!(
            "{},{},{},{},{:.6},{:.4},{:.2},{:.2},{}\n",
            trade.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            trade.pair,
            trade.side,
            trade.quantity,
            trade.price,
            trade.fee,
            trade.slippage,
            trade.execution_delay_ms,
            trade.id
        );
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.trade_log_file) 
        {
            // Write header if file is new
            if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
                let _ = writeln!(file, "timestamp,pair,side,quantity,price,fee,slippage,delay_ms,order_id");
            }
            let _ = write!(file, "{}", log_entry);
        }
    }

    fn log_portfolio_state(&self) {
        let summary = self.get_portfolio_summary();
        
        let log_entry = format!(
            "{},{:.2},{:.2},{:.2},{:.2},{:.2},{},{}\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            summary.total_value,
            summary.cash_balance,
            summary.unrealized_pnl,
            summary.realized_pnl,
            summary.total_return,
            summary.total_trades,
            summary.active_strategies
        );
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.portfolio_log_file) 
        {
            // Write header if file is new
            if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
                let _ = writeln!(file, "timestamp,total_value,cash_balance,unrealized_pnl,realized_pnl,total_return_pct,total_trades,active_strategies");
            }
            let _ = write!(file, "{}", log_entry);
        }
    }

    fn log_performance_update(&self) {
        // Log summary every 600 iterations (60 seconds at 10 Hz)
        static mut COUNTER: u32 = 0;
        unsafe {
            COUNTER += 1;
            if COUNTER % 600 == 0 {
                let summary = self.get_portfolio_summary();
                info!("💰 Portfolio: £{:.2} ({:+.2}%) | Trades: {} | Active Orders: {}", 
                    summary.total_value,
                    summary.total_return,
                    summary.total_trades,
                    self.count_active_orders()
                );
                
                // Log top performing pairs
                if !self.trade_history.is_empty() {
                    let mut pair_trades: HashMap<String, Vec<&SimulatedTrade>> = HashMap::new();
                    for trade in &self.trade_history {
                        pair_trades.entry(trade.pair.clone()).or_default().push(trade);
                    }
                    
                    info!("📊 Active pairs: {}", pair_trades.len());
                }
            }
        }
    }

    fn count_active_orders(&self) -> usize {
        self.strategies.values()
            .map(|s| s.active_orders.iter()
                .filter(|o| matches!(o.status, OrderStatus::Pending))
                .count())
            .sum()
    }

    /// Get current portfolio summary
    pub fn get_portfolio_summary(&self) -> PortfolioSummary {
        let total_value = self.portfolio.cash_balance + self.portfolio.unrealized_pnl;
        let total_return = (total_value - self.total_capital) / self.total_capital * 100.0;
        
        PortfolioSummary {
            total_value,
            cash_balance: self.portfolio.cash_balance,
            unrealized_pnl: self.portfolio.unrealized_pnl,
            realized_pnl: self.portfolio.realized_pnl,
            total_return,
            active_strategies: self.strategies.len(),
            total_trades: self.trade_history.len(),
            total_fees: self.portfolio.total_fees_paid,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PortfolioSummary {
    pub total_value: f64,
    pub cash_balance: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub total_return: f64,
    pub active_strategies: usize,
    pub total_trades: usize,
    pub total_fees: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_load_optimized_strategies() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_strategy.json");
        
        let strategy = OptimizedStrategy {
            trading_pair: "TESTGBP".to_string(),
            grid_levels: 10,
            grid_spacing: 0.02,
            expected_return: 0.15,
            total_trades: 5,
            win_rate: 0.6,
            sharpe_ratio: 1.2,
            max_drawdown: 0.05,
            total_fees: 10.0,
            markov_confidence: 0.75,
            generated_at: Utc::now(),
        };
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{}", serde_json::to_string_pretty(&strategy).unwrap()).unwrap();
        
        let mut engine = LiveTradingEngine::new(10000.0);
        let loaded_count = engine.load_optimized_strategies(dir.path()).unwrap();
        
        assert_eq!(loaded_count, 1);
        assert!(engine.strategies.contains_key("TESTGBP"));
    }
}