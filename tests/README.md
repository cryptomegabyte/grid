# E2E Tests for Grid Trading Bot

This directory contains comprehensive end-to-end tests for the grid trading bot.

## Test Structure

### Unit Tests (`tests/e2e_tests.rs`)

1. **Grid Trading Logic Tests**
   - `test_grid_trader_initialization()` - Verifies trader starts with clean state
   - `test_grid_setup_levels()` - Tests grid level calculation and spacing
   - `test_buy_signal_generation()` - Verifies buy signals trigger correctly
   - `test_sell_signal_generation()` - Verifies sell signals trigger correctly
   - `test_no_duplicate_signals()` - Ensures no duplicate signals at same level
   - `test_multiple_levels_signals()` - Tests cascading through multiple levels
   - `test_price_precision()` - Tests handling of precise price movements

2. **Data Processing Tests**
   - `test_kraken_ticker_parsing()` - Tests parsing of Kraken WebSocket messages
   - Validates correct extraction of price data from JSON

3. **Integration Tests**
   - `test_websocket_connection()` - **Live test** connecting to actual Kraken WebSocket
   - `test_end_to_end_grid_simulation()` - Complete grid trading simulation

## Running Tests

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
cargo test --test e2e_tests
```

## Test Features

### Grid Simulation
The e2e simulation test demonstrates a complete trading scenario:
- Sets up grid with 3 levels at £0.005 spacing
- Simulates realistic price movements
- Verifies correct signal generation
- Tracks all signals received

Example output:
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

### Live WebSocket Test
Tests actual connectivity to Kraken:
- Connects to `wss://ws.kraken.com`
- Subscribes to XRP/GBP ticker
- Verifies subscription confirmation
- Captures real price data
- Gracefully handles network failures

### Test Coverage
- **Grid Logic**: Complete coverage of trading signal generation
- **Data Parsing**: Validation of Kraken message format handling  
- **Integration**: Real-world connectivity and data flow
- **Edge Cases**: Precision handling, duplicate prevention, error scenarios

## Notes

- Integration tests may fail if network is unavailable (this is expected)
- Tests use smaller price spacing (£0.005) for faster signal generation
- All tests are designed to be deterministic and repeatable
- Live tests include timeout protection (10 seconds max)