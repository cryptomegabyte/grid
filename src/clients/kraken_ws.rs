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

    pub async fn subscribe_to_ohlc(&mut self, trading_pair: &str, interval: u32) -> Result<(), Box<dyn std::error::Error>> {
        let subscribe_message = json!({
            "event": "subscribe",
            "pair": [trading_pair],
            "subscription": {
                "name": "ohlc",
                "interval": interval
            }
        });
        
        self.ws_sender.send(Message::Text(subscribe_message.to_string())).await?;
        println!("📊 Subscribed to {} OHLC data ({}min)", trading_pair, interval);
        
        Ok(())
    }

    pub async fn subscribe_to_book(&mut self, trading_pair: &str, depth: u32) -> Result<(), Box<dyn std::error::Error>> {
        let subscribe_message = json!({
            "event": "subscribe",
            "pair": [trading_pair],
            "subscription": {
                "name": "book",
                "depth": depth
            }
        });
        
        self.ws_sender.send(Message::Text(subscribe_message.to_string())).await?;
        println!("📖 Subscribed to {} order book (depth: {})", trading_pair, depth);
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MarketData {
    pub pair: String,
    pub price: f64,
    pub bid: f64,
    pub ask: f64,
    pub volume_24h: f64,
    pub high_24h: f64,
    pub low_24h: f64,
    pub volatility: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct OrderBookLevel {
    pub price: f64,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub spread: f64,
}

#[derive(Debug, Clone)]
pub struct OHLCData {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub timestamp: u64,
}

pub fn parse_kraken_ticker(data: &Value) -> Option<MarketData> {
    // Check if this is a ticker update
    if let Some(channel_name) = data.get(2).and_then(|v| v.as_str()) {
        if channel_name == "ticker" {
            if let Some(ticker_data) = data.get(1) {
                let pair = data.get(3).and_then(|p| p.as_str()).unwrap_or("Unknown").to_string();
                
                // Extract comprehensive ticker data
                let price = ticker_data.get("c").and_then(|c| c.get(0)).and_then(|p| p.as_str())?
                    .parse::<f64>().ok()?;
                let bid = ticker_data.get("b").and_then(|b| b.get(0)).and_then(|p| p.as_str())?
                    .parse::<f64>().ok()?;
                let ask = ticker_data.get("a").and_then(|a| a.get(0)).and_then(|p| p.as_str())?
                    .parse::<f64>().ok()?;
                let volume_24h = ticker_data.get("v").and_then(|v| v.get(1)).and_then(|p| p.as_str())?
                    .parse::<f64>().unwrap_or(0.0);
                let high_24h = ticker_data.get("h").and_then(|h| h.get(1)).and_then(|p| p.as_str())?
                    .parse::<f64>().unwrap_or(price);
                let low_24h = ticker_data.get("l").and_then(|l| l.get(1)).and_then(|p| p.as_str())?
                    .parse::<f64>().unwrap_or(price);
                
                let volatility = if high_24h > low_24h {
                    (high_24h - low_24h) / price
                } else {
                    0.01 // Default 1% volatility
                };
                
                return Some(MarketData {
                    pair,
                    price,
                    bid,
                    ask,
                    volume_24h,
                    high_24h,
                    low_24h,
                    volatility,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                });
            }
        }
    }
    None
}

pub fn parse_kraken_ohlc(data: &Value) -> Option<OHLCData> {
    if let Some(channel_name) = data.get(2).and_then(|v| v.as_str()) {
        if channel_name == "ohlc-1" || channel_name.starts_with("ohlc-") {
            if let Some(ohlc_array) = data.get(1).and_then(|d| d.as_array()) {
                if ohlc_array.len() >= 8 {
                    let timestamp = ohlc_array[0].as_str()?.parse::<f64>().ok()? as u64;
                    let open = ohlc_array[1].as_str()?.parse::<f64>().ok()?;
                    let high = ohlc_array[2].as_str()?.parse::<f64>().ok()?;
                    let low = ohlc_array[3].as_str()?.parse::<f64>().ok()?;
                    let close = ohlc_array[4].as_str()?.parse::<f64>().ok()?;
                    let volume = ohlc_array[6].as_str()?.parse::<f64>().ok()?;
                    
                    return Some(OHLCData {
                        open,
                        high,
                        low,
                        close,
                        volume,
                        timestamp,
                    });
                }
            }
        }
    }
    None
}

pub fn parse_kraken_orderbook(data: &Value) -> Option<OrderBook> {
    if let Some(channel_name) = data.get(2).and_then(|v| v.as_str()) {
        if channel_name == "book-10" || channel_name.starts_with("book-") {
            if let Some(book_data) = data.get(1) {
                let mut bids = Vec::new();
                let mut asks = Vec::new();
                
                // Parse bids
                if let Some(bid_array) = book_data.get("b").and_then(|b| b.as_array()) {
                    for bid in bid_array {
                        if let Some(bid_array) = bid.as_array() {
                            if bid_array.len() >= 2 {
                                if let (Some(price_str), Some(volume_str)) = 
                                    (bid_array[0].as_str(), bid_array[1].as_str()) {
                                    if let (Ok(price), Ok(volume)) = 
                                        (price_str.parse::<f64>(), volume_str.parse::<f64>()) {
                                        bids.push(OrderBookLevel { price, volume });
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Parse asks
                if let Some(ask_array) = book_data.get("a").and_then(|a| a.as_array()) {
                    for ask in ask_array {
                        if let Some(ask_array) = ask.as_array() {
                            if ask_array.len() >= 2 {
                                if let (Some(price_str), Some(volume_str)) = 
                                    (ask_array[0].as_str(), ask_array[1].as_str()) {
                                    if let (Ok(price), Ok(volume)) = 
                                        (price_str.parse::<f64>(), volume_str.parse::<f64>()) {
                                        asks.push(OrderBookLevel { price, volume });
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Calculate spread
                let spread = if !bids.is_empty() && !asks.is_empty() {
                    asks[0].price - bids[0].price
                } else {
                    0.0
                };
                
                return Some(OrderBook {
                    bids,
                    asks,
                    spread,
                });
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