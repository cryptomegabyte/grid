# Grid Trading Bot

A simple cryptocurrency grid trading bot written in Rust, using Kraken's public WebSocket feed.

## Features

- **Real-time price monitoring** via Kraken WebSocket API
- **Grid trading strategy** with configurable levels and spacing
- **GBP trading pair** (XRP/GBP - Ripple vs British Pound)
- **Signal detection** for buy/sell opportunities
- **Minimal dependencies** and clean architecture

## Configuration

Key parameters can be adjusted in `src/main.rs`:

```rust
const GRID_LEVELS: usize = 5;        // Number of levels above/below current price
const GRID_SPACING: f64 = 0.01;      // £0.01 spacing between grid levels (XRP prices are lower)
const MIN_PRICE_CHANGE: f64 = 0.001; // Min price change to log (reduces spam)
```

## Running the Bot

```bash
cargo run
```

## How It Works

1. **Connects** to Kraken's WebSocket feed
2. **Subscribes** to XRP/GBP ticker data
3. **Sets up grid** with buy levels below and sell levels above current price
4. **Monitors** price movements and triggers signals when levels are hit
5. **Prints** trading signals to console (paper trading mode)

## Example Output

```
🚀 Starting Grid Trading Bot for XRP/GBP pair
📊 Grid Configuration: 5 levels, £0.01 spacing
✅ Connected to Kraken WebSocket
📡 Subscribed to XRP/GBP ticker data
💰 Current XRP/GBP price: £2.1176
🎯 Grid Setup Complete!
   📉 Buy levels:  ["£2.1076", "£2.0976", "£2.0876", "£2.0776", "£2.0676"]
   📈 Sell levels: ["£2.1276", "£2.1376", "£2.1476", "£2.1576", "£2.1676"]
🟢 BUY SIGNAL! Price £2.1070 hit buy level £2.1076
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

## Testing

The project includes comprehensive end-to-end tests covering all aspects of the grid trading bot.

### Running Tests

```bash
# Run all tests
make test

# Run with detailed output
make test-verbose

# Run only e2e tests
make test-e2e

# Run e2e tests with detailed output  
make test-e2e-verbose

# Or use cargo directly
cargo test
cargo test -- --nocapture
```

### Test Coverage

**Unit Tests (7 tests):**
- Grid initialization and setup
- Signal generation (buy/sell)  
- Duplicate signal prevention
- Multi-level signal cascading
- Price precision handling
- Kraken message parsing

**Integration Tests (2 tests):**
- Live WebSocket connection to Kraken
- End-to-end grid trading simulation

### Test Features

The e2e simulation demonstrates a complete trading scenario:
- Sets up grid with configurable levels and spacing
- Simulates realistic price movements
- Verifies correct signal generation
- Tracks all signals received

Example test output:
```
🎯 Grid simulation started
   Initial price: £1.5000
   Buy levels: [1.495, 1.49, 1.485]
   Sell levels: [1.505, 1.51, 1.515]
📈 Step 2: Price £1.4950
  🟢 BUY signal at £1.4950
📈 Step 5: Price £1.5050
  🔴 SELL signal at £1.5050
```

The live WebSocket test connects to actual Kraken infrastructure and verifies:
- WebSocket connection establishment
- Subscription to XRP/GBP ticker
- Real price data reception
- Graceful error handling

## Build Commands

The project includes a Makefile with convenient commands:

```bash
make              # Show help menu
make build        # Build the project
make run          # Run the grid trading bot
make dev          # Run with debug logging
make release      # Build optimized version
make clean        # Clean build artifacts
make fmt          # Format code
make clippy       # Run linter
make test         # Run all tests
make info         # Show project info
```

## Safety Note

This bot currently operates in **paper trading mode** (no actual trades). Always test thoroughly before implementing actual trading functionality.