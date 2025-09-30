use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::{json, Value};
use futures_util::{SinkExt, StreamExt};

const KRAKEN_WS_URL: &str = "wss://ws.kraken.com";
const TRADING_PAIR: &str = "XRP/GBP";  // Bitcoin vs British Pound

// Grid trading configuration - easily adjustable
const GRID_LEVELS: usize = 5;      // Number of buy/sell levels above and below current price
const GRID_SPACING: f64 = 0.01;    // £0.01 spacing between grid levels (XRP prices)
const MIN_PRICE_CHANGE: f64 = 0.001; // Only log price updates if change is > £0.001

// Simple market state for Markov model
#[derive(Debug, Clone, Copy, PartialEq)]
enum MarketState {
    TrendingUp,    // Price generally increasing
    TrendingDown,  // Price generally decreasing  
    Ranging,       // Price moving sideways
}

struct GridTrader {
    current_price: f64,
    buy_levels: Vec<f64>,
    sell_levels: Vec<f64>,
    last_triggered_level: Option<f64>,
    last_logged_price: f64,  // To reduce spam in console
    // Simple Markov enhancement
    price_history: Vec<f64>,  // Keep last 10 prices for state detection
    current_state: MarketState,
    base_grid_spacing: f64,   // Original spacing
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Grid Trading Bot for {} pair", TRADING_PAIR);
    println!("📊 Grid Configuration: {} levels, £{} spacing", GRID_LEVELS, GRID_SPACING);
    
    // Initialize grid trader
    let mut grid_trader = GridTrader {
        current_price: 0.0,
        buy_levels: Vec::new(),
        sell_levels: Vec::new(),
        last_triggered_level: None,
        last_logged_price: 0.0,
        price_history: Vec::new(),
        current_state: MarketState::Ranging,
        base_grid_spacing: GRID_SPACING,
    };
    
    // Connect to Kraken WebSocket
    let (ws_stream, _) = connect_async(KRAKEN_WS_URL).await?;
    println!("✅ Connected to Kraken WebSocket");
    
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Subscribe to ticker data for XBT/GBP
    let subscribe_message = json!({
        "event": "subscribe",
        "pair": [TRADING_PAIR],
        "subscription": {
            "name": "ticker"
        }
    });
    
    ws_sender.send(Message::Text(subscribe_message.to_string())).await?;
    println!("📡 Subscribed to {} ticker data", TRADING_PAIR);
    
    // Listen for messages
    while let Some(message) = ws_receiver.next().await {
        match message? {
            Message::Text(text) => {
                // Parse the JSON message
                if let Ok(data) = serde_json::from_str::<Value>(&text) {
                    handle_message(data, &mut grid_trader).await;
                }
            }
            Message::Close(_) => {
                println!("⚠️ WebSocket connection closed");
                break;
            }
            _ => {}
        }
    }
    
    Ok(())
}

async fn handle_message(data: Value, grid_trader: &mut GridTrader) {
    // Check if this is a ticker update
    if let Some(channel_name) = data.get(2).and_then(|v| v.as_str()) {
        if channel_name == "ticker" {
            if let Some(ticker_data) = data.get(1) {
                // Extract the current price (last traded price)
                if let Some(price_str) = ticker_data.get("c").and_then(|c| c.get(0)).and_then(|p| p.as_str()) {
                    if let Ok(current_price) = price_str.parse::<f64>() {
                        // Only log price if it changed significantly
                        if (current_price - grid_trader.last_logged_price).abs() > MIN_PRICE_CHANGE {
                            println!("💰 Current {} price: £{:.2}", TRADING_PAIR, current_price);
                            grid_trader.last_logged_price = current_price;
                        }
                        
                        // Update grid trader with new price
                        update_grid_trader(grid_trader, current_price);
                    }
                }
            }
        }
    }
    
    // Handle subscription confirmations and other events
    if let Some(event) = data.get("event").and_then(|e| e.as_str()) {
        match event {
            "subscriptionStatus" => {
                if let Some(status) = data.get("status").and_then(|s| s.as_str()) {
                    println!("📊 Subscription status: {}", status);
                }
            }
            "systemStatus" => {
                if let Some(status) = data.get("status").and_then(|s| s.as_str()) {
                    println!("🔧 System status: {}", status);
                }
            }
            _ => {}
        }
    }
}

impl GridTrader {
    fn setup_grid(&mut self, center_price: f64) {
        self.current_price = center_price;
        self.buy_levels.clear();
        self.sell_levels.clear();
        
        // Get adjusted spacing based on market state
        let adjusted_spacing = self.get_adjusted_spacing();
        
        // Create buy levels below current price
        for i in 1..=GRID_LEVELS {
            let buy_level = center_price - (i as f64 * adjusted_spacing);
            self.buy_levels.push(buy_level);
        }
        
        // Create sell levels above current price
        for i in 1..=GRID_LEVELS {
            let sell_level = center_price + (i as f64 * adjusted_spacing);
            self.sell_levels.push(sell_level);
        }
        
        println!("🎯 Grid Setup Complete! (State: {:?}, Spacing: £{:.4})", self.current_state, adjusted_spacing);
        println!("   📉 Buy levels:  {:?}", self.buy_levels.iter().map(|&x| format!("£{:.4}", x)).collect::<Vec<_>>());
        println!("   📈 Sell levels: {:?}", self.sell_levels.iter().map(|&x| format!("£{:.4}", x)).collect::<Vec<_>>());
    }
    
    // Simple market state detection based on recent price movements
    fn detect_market_state(&mut self, new_price: f64) {
        // Add new price to history
        self.price_history.push(new_price);
        
        // Keep only last 10 prices for analysis
        if self.price_history.len() > 10 {
            self.price_history.remove(0);
        }
        
        // Need at least 5 prices to detect state
        if self.price_history.len() < 5 {
            return;
        }
        
        let first_price = self.price_history[0];
        let last_price = *self.price_history.last().unwrap();
        let price_change_pct = (last_price - first_price) / first_price;
        
        // Calculate volatility (simple standard deviation)
        let avg_price: f64 = self.price_history.iter().sum::<f64>() / self.price_history.len() as f64;
        let variance: f64 = self.price_history.iter().map(|&p| (p - avg_price).powi(2)).sum::<f64>() / self.price_history.len() as f64;
        let volatility = variance.sqrt() / avg_price; // Coefficient of variation
        
        let old_state = self.current_state;
        
        // Simple state detection logic
        if price_change_pct > 0.005 && volatility < 0.02 {  // 0.5% increase, low volatility
            self.current_state = MarketState::TrendingUp;
        } else if price_change_pct < -0.005 && volatility < 0.02 {  // 0.5% decrease, low volatility
            self.current_state = MarketState::TrendingDown;
        } else {
            self.current_state = MarketState::Ranging;  // High volatility or small moves
        }
        
        // Log state changes
        if old_state != self.current_state {
            println!("🔄 Market state changed: {:?} → {:?} (Price change: {:.2}%, Volatility: {:.2}%)", 
                     old_state, self.current_state, price_change_pct * 100.0, volatility * 100.0);
            
            // Rebuild grid with new spacing
            if !self.buy_levels.is_empty() {
                self.setup_grid(new_price);
            }
        }
    }
    
    // Get adjusted grid spacing based on current market state
    fn get_adjusted_spacing(&self) -> f64 {
        match self.current_state {
            MarketState::TrendingUp | MarketState::TrendingDown => {
                // Wider spacing in trending markets to avoid too many signals
                self.base_grid_spacing * 1.5
            }
            MarketState::Ranging => {
                // Normal spacing in ranging markets
                self.base_grid_spacing
            }
        }
    }
    
    fn check_grid_signals(&mut self, current_price: f64) {
        // Check if price hit any buy levels
        for &buy_level in &self.buy_levels {
            if current_price <= buy_level && self.last_triggered_level != Some(buy_level) {
                println!("🟢 BUY SIGNAL! Price £{:.4} hit buy level £{:.4}", current_price, buy_level);
                self.last_triggered_level = Some(buy_level);
                return;
            }
        }
        
        // Check if price hit any sell levels
        for &sell_level in &self.sell_levels {
            if current_price >= sell_level && self.last_triggered_level != Some(sell_level) {
                println!("🔴 SELL SIGNAL! Price £{:.4} hit sell level £{:.4}", current_price, sell_level);
                self.last_triggered_level = Some(sell_level);
                return;
            }
        }
    }
}

fn update_grid_trader(grid_trader: &mut GridTrader, current_price: f64) {
    // Update market state detection with new price
    grid_trader.detect_market_state(current_price);
    
    // Initialize grid if this is the first price update
    if grid_trader.buy_levels.is_empty() {
        grid_trader.setup_grid(current_price);
    }
    
    // Check for trading signals
    grid_trader.check_grid_signals(current_price);
    grid_trader.current_price = current_price;
}
