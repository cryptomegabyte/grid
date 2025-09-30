use grid_trading_bot::{
    GridTrader, KrakenWebSocketClient, parse_kraken_ticker, handle_kraken_event,
    Config
};
use tokio_tungstenite::tungstenite::protocol::Message;
use serde_json::Value;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load_or_create("config.toml")?;
    
    println!("🚀 Starting Grid Trading Bot for {} pair", config.trading.trading_pair);
    println!("📊 Grid Configuration: {} levels, £{:.4} base spacing", 
             config.trading.grid_levels, config.trading.grid_spacing);
    
    // Initialize grid trader with configuration
    let mut grid_trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Connect to Kraken WebSocket
    let mut ws_client = KrakenWebSocketClient::connect(&config.trading.kraken_ws_url).await?;
    
    // Subscribe to ticker data
    ws_client.subscribe_to_ticker(&config.trading.trading_pair).await?;
    
    // Listen for messages
    while let Some(message) = ws_client.ws_receiver.next().await {
        match message? {
            Message::Text(text) => {
                // Parse the JSON message
                if let Ok(data) = serde_json::from_str::<Value>(&text) {
                    handle_message(data, &mut grid_trader, &config).await;
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

async fn handle_message(data: Value, grid_trader: &mut GridTrader, config: &Config) {
    // Try to parse ticker data
    if let Some(current_price) = parse_kraken_ticker(&data) {
        // Only log price if it changed significantly
        if grid_trader.should_log_price(current_price, config.trading.min_price_change) {
            if config.logging.enable_price_logging {
                println!("💰 Current {} price: £{:.4}", config.trading.trading_pair, current_price);
            }
            grid_trader.update_logged_price(current_price);
        }
        
        // Update grid trader with new price (includes market analysis)
        let _signal = grid_trader.update_with_price(current_price);
        // Signal handling could be added here for actual trading
    }
    
    // Handle subscription confirmations and other events
    handle_kraken_event(&data);
}
