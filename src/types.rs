// Common types used across the application

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarketState {
    TrendingUp,    // Price generally increasing
    TrendingDown,  // Price generally decreasing  
    Ranging,       // Price moving sideways
}

#[derive(Debug, PartialEq)]
pub enum GridSignal {
    Buy(f64),   // Buy signal with price level
    Sell(f64),  // Sell signal with price level
    None,       // No signal
}

// Configuration constants
pub const KRAKEN_WS_URL: &str = "wss://ws.kraken.com";
pub const TRADING_PAIR: &str = "XRP/GBP";
pub const GRID_LEVELS: usize = 5;
pub const GRID_SPACING: f64 = 0.01;
pub const MIN_PRICE_CHANGE: f64 = 0.001;