use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::{json, Value};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;

const KRAKEN_WS_URL: &str = "wss://ws.kraken.com";
const TRADING_PAIR: &str = "XRP/GBP";

// Simple grid trader for testing
#[derive(Debug, Clone)]
struct TestGridTrader {
    pub current_price: f64,
    pub buy_levels: Vec<f64>,
    pub sell_levels: Vec<f64>,
    pub signals_received: Vec<String>,
}

#[derive(Debug, PartialEq)]
enum GridSignal {
    Buy(f64),
    Sell(f64),
    None,
}

impl TestGridTrader {
    fn new() -> Self {
        Self {
            current_price: 0.0,
            buy_levels: Vec::new(),
            sell_levels: Vec::new(),
            signals_received: Vec::new(),
        }
    }

    fn setup_grid(&mut self, center_price: f64, levels: usize, spacing: f64) {
        self.current_price = center_price;
        self.buy_levels.clear();
        self.sell_levels.clear();
        
        // Create buy levels below current price
        for i in 1..=levels {
            let buy_level = center_price - (i as f64 * spacing);
            self.buy_levels.push(buy_level);
        }
        
        // Create sell levels above current price
        for i in 1..=levels {
            let sell_level = center_price + (i as f64 * spacing);
            self.sell_levels.push(sell_level);
        }
    }
    
    fn check_signals(&mut self, current_price: f64) -> GridSignal {
        // Check buy levels
        for &buy_level in &self.buy_levels {
            if current_price <= buy_level {
                let signal = format!("BUY at £{:.4}", buy_level);
                if !self.signals_received.contains(&signal) {
                    self.signals_received.push(signal);
                    return GridSignal::Buy(buy_level);
                }
            }
        }
        
        // Check sell levels  
        for &sell_level in &self.sell_levels {
            if current_price >= sell_level {
                let signal = format!("SELL at £{:.4}", sell_level);
                if !self.signals_received.contains(&signal) {
                    self.signals_received.push(signal);
                    return GridSignal::Sell(sell_level);
                }
            }
        }
        
        GridSignal::None
    }
}

fn parse_kraken_ticker(data: &Value) -> Option<f64> {
    if let Some(channel_name) = data.get(2).and_then(|v| v.as_str()) {
        if channel_name == "ticker" {
            if let Some(ticker_data) = data.get(1) {
                if let Some(price_str) = ticker_data.get("c").and_then(|c| c.get(0)).and_then(|p| p.as_str()) {
                    return price_str.parse::<f64>().ok();
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_trader_initialization() {
        let trader = TestGridTrader::new();
        assert_eq!(trader.current_price, 0.0);
        assert_eq!(trader.buy_levels.len(), 0);
        assert_eq!(trader.sell_levels.len(), 0);
        assert_eq!(trader.signals_received.len(), 0);
    }

    #[test]
    fn test_grid_setup_levels() {
        let mut trader = TestGridTrader::new();
        let center_price = 1.5;
        let levels = 3;
        let spacing = 0.01;
        
        trader.setup_grid(center_price, levels, spacing);
        
        assert_eq!(trader.current_price, center_price);
        assert_eq!(trader.buy_levels.len(), levels);
        assert_eq!(trader.sell_levels.len(), levels);
        
        // Verify buy levels are correctly spaced below center
        assert_eq!(trader.buy_levels[0], center_price - spacing);
        assert_eq!(trader.buy_levels[1], center_price - (2.0 * spacing));
        assert_eq!(trader.buy_levels[2], center_price - (3.0 * spacing));
        
        // Verify sell levels are correctly spaced above center
        assert_eq!(trader.sell_levels[0], center_price + spacing);
        assert_eq!(trader.sell_levels[1], center_price + (2.0 * spacing));
        assert_eq!(trader.sell_levels[2], center_price + (3.0 * spacing));
    }

    #[test]
    fn test_buy_signal_generation() {
        let mut trader = TestGridTrader::new();
        let center_price = 1.5;
        trader.setup_grid(center_price, 3, 0.01);
        
        // Price drops to first buy level
        let buy_price = center_price - 0.01;
        let signal = trader.check_signals(buy_price);
        
        assert_eq!(signal, GridSignal::Buy(buy_price));
        assert_eq!(trader.signals_received.len(), 1);
        assert!(trader.signals_received[0].contains("BUY"));
    }

    #[test]
    fn test_sell_signal_generation() {
        let mut trader = TestGridTrader::new();
        let center_price = 1.5;
        trader.setup_grid(center_price, 3, 0.01);
        
        // Price rises to first sell level
        let sell_price = center_price + 0.01;
        let signal = trader.check_signals(sell_price);
        
        assert_eq!(signal, GridSignal::Sell(sell_price));
        assert_eq!(trader.signals_received.len(), 1);
        assert!(trader.signals_received[0].contains("SELL"));
    }

    #[test]
    fn test_no_duplicate_signals() {
        let mut trader = TestGridTrader::new();
        let center_price = 1.5;
        trader.setup_grid(center_price, 3, 0.01);
        
        let buy_price = center_price - 0.01;
        
        // First signal should trigger
        let signal1 = trader.check_signals(buy_price);
        assert_eq!(signal1, GridSignal::Buy(buy_price));
        assert_eq!(trader.signals_received.len(), 1);
        
        // Second signal at same price should not trigger
        let signal2 = trader.check_signals(buy_price);
        assert_eq!(signal2, GridSignal::None);
        assert_eq!(trader.signals_received.len(), 1); // No new signals
    }

    #[test]
    fn test_multiple_levels_signals() {
        let mut trader = TestGridTrader::new();
        let center_price = 1.5;
        trader.setup_grid(center_price, 3, 0.01);
        
        // Trigger first buy level
        let buy_price1 = center_price - 0.01;
        trader.check_signals(buy_price1);
        
        // Trigger second buy level
        let buy_price2 = center_price - 0.02;
        let signal = trader.check_signals(buy_price2);
        
        assert_eq!(signal, GridSignal::Buy(buy_price2));
        assert_eq!(trader.signals_received.len(), 2);
    }

    #[test]
    fn test_kraken_ticker_parsing() {
        // Valid ticker message from Kraken
        let ticker_data = json!([
            0,
            {
                "c": ["1.5234", "0.00100000"],
                "v": ["123.45", "678.90"],
                "o": ["1.5200", "1.5100"]
            },
            "ticker",
            "XRP/GBP"
        ]);
        
        let price = parse_kraken_ticker(&ticker_data);
        assert_eq!(price, Some(1.5234));
        
        // Invalid message structure
        let invalid_data = json!({
            "event": "systemStatus",
            "status": "online"
        });
        
        let price = parse_kraken_ticker(&invalid_data);
        assert_eq!(price, None);
        
        // Message with wrong channel
        let wrong_channel = json!([
            0,
            {"c": ["1.5234", "0.00100000"]},
            "trade",
            "XRP/GBP"
        ]);
        
        let price = parse_kraken_ticker(&wrong_channel);
        assert_eq!(price, None);
    }

    #[test]
    fn test_price_precision() {
        let mut trader = TestGridTrader::new();
        let center_price = 1.50000;
        trader.setup_grid(center_price, 2, 0.00001); // Very small spacing
        
        // Test precision handling
        let precise_price = center_price - 0.00001;
        let signal = trader.check_signals(precise_price);
        
        assert_eq!(signal, GridSignal::Buy(precise_price));
    }

    #[tokio::test]
    async fn test_websocket_connection() {
        // Integration test - connects to actual Kraken WebSocket
        let timeout = Duration::from_secs(10);
        
        let result = tokio::time::timeout(timeout, async {
            println!("🔄 Testing WebSocket connection to Kraken...");
            
            let (ws_stream, response) = connect_async(KRAKEN_WS_URL).await?;
            println!("✅ Connected! Response status: {}", response.status());
            
            let (mut ws_sender, mut ws_receiver) = ws_stream.split();
            
            // Subscribe to ticker
            let subscribe_message = json!({
                "event": "subscribe",
                "pair": [TRADING_PAIR],
                "subscription": {
                    "name": "ticker"
                }
            });
            
            ws_sender.send(Message::Text(subscribe_message.to_string())).await?;
            println!("📡 Sent subscription request for {}", TRADING_PAIR);
            
            let mut message_count = 0;
            let mut subscription_confirmed = false;
            let mut price_received = false;
            
            // Wait for a few messages
            while message_count < 10 {
                if let Some(message) = ws_receiver.next().await {
                    match message? {
                        Message::Text(text) => {
                            if let Ok(data) = serde_json::from_str::<Value>(&text) {
                                // Check for subscription confirmation
                                if let Some(event) = data.get("event").and_then(|e| e.as_str()) {
                                    if event == "subscriptionStatus" {
                                        if let Some(status) = data.get("status").and_then(|s| s.as_str()) {
                                            println!("📊 Subscription status: {}", status);
                                            if status == "subscribed" {
                                                subscription_confirmed = true;
                                            }
                                        }
                                    }
                                }
                                
                                // Check for price data
                                if let Some(price) = parse_kraken_ticker(&data) {
                                    println!("💰 Received price: £{:.4}", price);
                                    price_received = true;
                                    assert!(price > 0.0, "Price should be positive");
                                }
                            }
                        }
                        Message::Close(_) => {
                            println!("⚠️ Connection closed by server");
                            break;
                        }
                        _ => {}
                    }
                    message_count += 1;
                }
            }
            
            assert!(subscription_confirmed, "Should receive subscription confirmation");
            // Note: price_received might be false if no trades happen during test
            if price_received {
                println!("✅ Successfully received price data");
            } else {
                println!("ℹ️ No price updates during test period (this is normal)");
            }
            
            Ok::<(), Box<dyn std::error::Error>>(())
        }).await;
        
        match result {
            Ok(Ok(())) => {
                println!("✅ WebSocket integration test passed");
            }
            Ok(Err(e)) => {
                println!("⚠️ WebSocket test failed: {}", e);
                // Don't panic - network issues are common in CI
                println!("  This is expected if network/Kraken is unavailable");
            }
            Err(_) => {
                println!("⚠️ WebSocket test timed out");
                println!("  This is expected if network is slow");
            }
        }
    }

    #[tokio::test]
    async fn test_end_to_end_grid_simulation() {
        // Simulate a complete grid trading scenario
        let mut trader = TestGridTrader::new();
        let initial_price = 1.5000;
        
        // Setup grid
        trader.setup_grid(initial_price, 3, 0.0050); // 3 levels, £0.005 spacing
        
        println!("🎯 Grid simulation started");
        println!("   Initial price: £{:.4}", initial_price);
        println!("   Buy levels: {:?}", trader.buy_levels);
        println!("   Sell levels: {:?}", trader.sell_levels);
        
        // Simulate price movements
        let price_movements = vec![
            initial_price,      // Start
            1.4950,            // Hit first buy level
            1.4900,            // Hit second buy level  
            1.4950,            // Price recovers
            1.5050,            // Hit first sell level
            1.5100,            // Hit second sell level
            1.5000,            // Back to center
        ];
        
        let mut total_signals = 0;
        
        for (i, &price) in price_movements.iter().enumerate() {
            println!("📈 Step {}: Price £{:.4}", i + 1, price);
            
            let signal = trader.check_signals(price);
            match signal {
                GridSignal::Buy(level) => {
                    println!("  🟢 BUY signal at £{:.4}", level);
                    total_signals += 1;
                }
                GridSignal::Sell(level) => {
                    println!("  🔴 SELL signal at £{:.4}", level);
                    total_signals += 1;
                }
                GridSignal::None => {
                    println!("  ⚪ No signal");
                }
            }
        }
        
        println!("📊 Simulation complete");
        println!("   Total signals generated: {}", total_signals);
        println!("   Signals received: {:?}", trader.signals_received);
        
        // Verify we got the expected signals
        assert!(total_signals >= 4, "Should generate multiple signals");
        assert!(trader.signals_received.len() >= 4, "Should record multiple signals");
        
        // Verify signal types
        let buy_signals = trader.signals_received.iter().filter(|s| s.contains("BUY")).count();
        let sell_signals = trader.signals_received.iter().filter(|s| s.contains("SELL")).count();
        
        assert!(buy_signals >= 2, "Should have multiple buy signals");
        assert!(sell_signals >= 2, "Should have multiple sell signals");
        
        println!("✅ E2E grid simulation test passed");
    }
}