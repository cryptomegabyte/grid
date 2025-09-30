// WebSocket client for Kraken connection

use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::{json, Value};
use futures_util::{SinkExt, StreamExt};

pub struct KrakenWebSocketClient {
    pub ws_sender: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        Message
    >,
    pub ws_receiver: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>
    >,
}

impl KrakenWebSocketClient {
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(url).await?;
        println!("✅ Connected to Kraken WebSocket");
        
        let (ws_sender, ws_receiver) = ws_stream.split();
        
        Ok(Self {
            ws_sender,
            ws_receiver,
        })
    }

    pub async fn subscribe_to_ticker(&mut self, trading_pair: &str) -> Result<(), Box<dyn std::error::Error>> {
        let subscribe_message = json!({
            "event": "subscribe",
            "pair": [trading_pair],
            "subscription": {
                "name": "ticker"
            }
        });
        
        self.ws_sender.send(Message::Text(subscribe_message.to_string())).await?;
        println!("📡 Subscribed to {} ticker data", trading_pair);
        
        Ok(())
    }
}

pub fn parse_kraken_ticker(data: &Value) -> Option<f64> {
    // Check if this is a ticker update
    if let Some(channel_name) = data.get(2).and_then(|v| v.as_str()) {
        if channel_name == "ticker" {
            if let Some(ticker_data) = data.get(1) {
                // Extract the current price (last traded price)
                if let Some(price_str) = ticker_data.get("c").and_then(|c| c.get(0)).and_then(|p| p.as_str()) {
                    return price_str.parse::<f64>().ok();
                }
            }
        }
    }
    None
}

pub fn handle_kraken_event(data: &Value) {
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