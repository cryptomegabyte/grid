# Grid Trading Bot

A simple cryptocurrency grid trading bot written in Rust, using Kraken's public WebSocket feed.

## Features

- **Real-time price monitoring** via Kraken WebSocket API
- **Grid trading strategy** with configurable levels and spacing
- **GBP trading pair** (XBT/GBP - Bitcoin vs British Pound)
- **Signal detection** for buy/sell opportunities
- **Minimal dependencies** and clean architecture

## Configuration

Key parameters can be adjusted in `src/main.rs`:

```rust
const GRID_LEVELS: usize = 5;        // Number of levels above/below current price
const GRID_SPACING: f64 = 500.0;     // £500 spacing between grid levels  
const MIN_PRICE_CHANGE: f64 = 10.0;  // Min price change to log (reduces spam)
```

## Running the Bot

```bash
cargo run
```

## How It Works

1. **Connects** to Kraken's WebSocket feed
2. **Subscribes** to XBT/GBP ticker data
3. **Sets up grid** with buy levels below and sell levels above current price
4. **Monitors** price movements and triggers signals when levels are hit
5. **Prints** trading signals to console (paper trading mode)

## Example Output

```
🚀 Starting Grid Trading Bot for XBT/GBP pair
📊 Grid Configuration: 5 levels, £500 spacing
✅ Connected to Kraken WebSocket
📡 Subscribed to XBT/GBP ticker data
💰 Current XBT/GBP price: £84,485.00
🎯 Grid Setup Complete!
   📉 Buy levels:  ["£83985.00", "£83485.00", "£82985.00", "£82485.00", "£81985.00"]
   📈 Sell levels: ["£84985.00", "£85485.00", "£85985.00", "£86485.00", "£86985.00"]
🟢 BUY SIGNAL! Price £83980.00 hit buy level £83985.00
```

## Next Steps

This is a foundation that can be extended with:
- Multiple trading pairs
- Dynamic grid adjustment
- Risk management
- Actual order placement via Kraken's private API
- Profit/loss tracking
- Configuration files
- Logging to files
- Database persistence

## Safety Note

This bot currently operates in **paper trading mode** (no actual trades). Always test thoroughly before implementing actual trading functionality.