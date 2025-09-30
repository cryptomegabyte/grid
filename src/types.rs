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