use grid_trading_bot::{
    GridTrader, KrakenWebSocketClient, parse_kraken_ticker, handle_kraken_event,
    KRAKEN_WS_URL, TRADING_PAIR, GRID_SPACING, MIN_PRICE_CHANGE
};
use tokio_tungstenite::tungstenite::protocol::Message;
use serde_json::Value;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Grid Trading Bot for {} pair", TRADING_PAIR);
    println!("📊 Grid Configuration: £{:.4} base spacing", GRID_SPACING);
    
    // Initialize grid trader with Markov enhancement
    let mut grid_trader = GridTrader::new(GRID_SPACING);
    
    // Connect to Kraken WebSocket
    let mut ws_client = KrakenWebSocketClient::connect(KRAKEN_WS_URL).await?;
    
    // Subscribe to ticker data
    ws_client.subscribe_to_ticker(TRADING_PAIR).await?;
    
    // Listen for messages
    while let Some(message) = ws_client.ws_receiver.next().await {
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
    // Try to parse ticker data
    if let Some(current_price) = parse_kraken_ticker(&data) {
        // Only log price if it changed significantly
        if grid_trader.should_log_price(current_price, MIN_PRICE_CHANGE) {
            println!("💰 Current {} price: £{:.4}", TRADING_PAIR, current_price);
            grid_trader.update_logged_price(current_price);
        }
        
        // Update grid trader with new price (includes Markov analysis)
        let _signal = grid_trader.update_with_price(current_price);
        // Signal handling could be added here for actual trading
    }
    
    // Handle subscription confirmations and other events
    handle_kraken_event(&data);
}
