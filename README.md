# Grid Trading Bot

Professional cryptocurrency grid trading system with automated strategy optimization and backtesting.

## Quick Start

```bash
# Clone and build
git clone https://github.com/cryptomegabyte/grid.git
cd grid
cargo build --release

# Initialize workspace
./target/release/grid-bot init

# Edit config.toml with your settings
nano config.toml

# Run optimization (no API keys needed)
make backtest

# Test with paper trading (no API keys needed)
./target/release/grid-bot trade start --dry-run --capital 500
```

## Features

- **Automated Optimization**: Find best parameters for your trading pairs
- **Vectorized Backtesting**: Fast historical data analysis using Polars
- **Paper Trading**: Test strategies risk-free before going live
- **SQLite Database**: Persistent storage for strategies and trade history
- **Progress Bars**: Real-time feedback during optimization
- **Pre-flight Validation**: Catch errors before execution

## Commands

### Setup

```bash
# Initialize configuration and database
grid-bot init

# Check system status
grid-bot status
```

### Strategy Management

```bash
# List all strategies
grid-bot strategy list

# Show strategy details
grid-bot strategy show ETHGBP

# Export strategy
grid-bot strategy export ETHGBP > my_strategy.json
```

### Optimization

```bash
# Optimize all GBP pairs
grid-bot optimize all --limit 10 --iterations 20

# Optimize specific pair
grid-bot optimize pair ETHGBP --iterations 50 --comprehensive
```

### Backtesting

```bash
# Quick demo backtest
grid-bot backtest demo ETHGBP

# Custom backtest
grid-bot backtest run ETHGBP --levels 10 --spacing 0.02

# Scan multiple pairs
grid-bot backtest scan --limit 5
```

### Trading

```bash
# Paper trading (no API keys needed)
grid-bot trade start --dry-run --capital 500

# Live trading (requires API keys)
grid-bot trade start --capital 500 --hours 8

# Trade specific pairs
grid-bot trade start --pairs ETHGBP,BTCGBP --dry-run
```

## Makefile Commands

Simple commands for common workflows:

```bash
make              # Show available commands
make trade-sim    # Paper trading with simulation engine (30 min, £500)
make trade        # Interactive: choose mode, capital, and duration
make trade-live   # Live trading without simulation (use with caution!)
make backtest     # Run optimization on 10 pairs
make test         # Run all tests
make test-sim     # Run simulation engine tests only
make build        # Build release binaries
make clean        # Clean build artifacts
```

## Simulation Engine 🎮

The simulation engine provides a fully-fledged local exchange simulator that replicates Kraken's order matching and execution behavior. **Now fully integrated into the live trading system!**

### Usage

```bash
# Paper trading with simulation engine (recommended)
make trade-sim

# Interactive mode - choose your settings
make trade

# Custom parameters
cargo run --bin trade start --simulate --capital 1000 --strategies-dir strategies --hours 2
```

### Key Features

- **Local Order Book**: Real-time order book state from Kraken WebSocket
- **Realistic Matching**: Price-time priority with partial fills and market impact
- **Execution Simulation**: Latency (50-200ms), slippage, fees (0.16% maker, 0.26% taker)
- **Market Impact Analysis**: Pre-calculate impact before execution
- **Multiple Slippage Models**: Fixed, SquareRoot, Linear, and Realistic
- **Performance Tracking**: Comprehensive statistics and metrics
- **Organized Logging**: Automatic CSV logs in `logs/portfolio/` and `logs/trades/`

### Trading Modes

**Paper Trading (with simulation)**:

```bash
cargo run --bin trade start --simulate --capital 500 --strategies-dir strategies --minutes 30
```

**Live Trading (without simulation)**:

```bash
cargo run --bin trade start --capital 500 --strategies-dir strategies --minutes 30
```

### How It Works

When `--simulate` is enabled:

1. ✅ Connects to Kraken WebSocket for real-time market data
2. ✅ Maintains local order books for all trading pairs
3. ✅ Simulates realistic order matching with market impact
4. ✅ Applies latency, slippage, and Kraken's actual fee structure
5. ✅ Logs all trades and portfolio changes to CSV files
6. ✅ **Zero risk** - No real orders sent to exchange

### Code Example

```rust
use grid_trading_bot::simulation::{
    SimulationEngine, OrderBookSnapshot,
    matching_engine::{SimulatedOrder, OrderSide, OrderType},
};

// Create engine
let mut engine = SimulationEngine::kraken_simulator();

// Initialize order book
let snapshot = OrderBookSnapshot {
    pair: "ETHGBP".to_string(),
    bids: vec![(2500.0, 2.0), (2499.0, 3.0)],
    asks: vec![(2501.0, 2.0), (2502.0, 3.0)],
    timestamp: Utc::now(),
};
engine.initialize_order_book("ETHGBP".to_string(), snapshot);

// Execute order
let order = SimulatedOrder {
    id: "order-1".to_string(),
    pair: "ETHGBP".to_string(),
    side: OrderSide::Buy,
    order_type: OrderType::Market,
    price: None,
    quantity: 3.0,
    timestamp: Utc::now(),
};

let result = engine.execute_order(order)?;
println!("Filled: {} @ £{:.2}", result.total_filled, result.average_price);
println!("Fees: £{:.2} | Slippage: £{:.2}", result.total_fees, result.total_slippage);
```

### Testing

```bash
# Run simulation engine tests
make test-sim

# Run all tests
make test
```

## Configuration

Edit `config.toml` to customize your settings:

```toml
[api]
api_key = "YOUR_API_KEY_HERE"       # Only needed for live trading
api_secret = "YOUR_API_SECRET_HERE" # Only needed for live trading

[trading]
default_capital = 500.0              # Starting capital
default_grid_levels = 10             # Number of grid levels
default_grid_spacing = 0.02          # Spacing between levels (2%)
max_position_size = 0.1              # Max 10% per position
risk_limit_per_trade = 0.02          # Risk 2% per trade

[backtesting]
default_lookback_days = 90           # Historical data period
default_timeframe_minutes = 60       # 1-hour candles

[optimization]
grid_levels_range = [5, 15]          # Test 5-15 levels
grid_spacing_range = [0.01, 0.05]    # Test 1-5% spacing
iterations = 20                       # Optimization iterations
```

## Architecture

```text
grid-trading-bot/
├── src/
│   ├── bin/
│   │   └── grid-bot.rs          # Unified CLI entry point
│   ├── cli/
│   │   ├── backtest_commands.rs # Backtest command handlers
│   │   └── trade_commands.rs    # Trade command handlers
│   ├── core/
│   │   ├── grid_trader.rs       # Core trading logic
│   │   ├── market_state.rs      # Market analysis
│   │   └── live_trading.rs      # Live trading engine
│   ├── simulation/              # 🆕 Simulation Engine
│   │   ├── order_book.rs        # Local order book manager
│   │   ├── matching_engine.rs   # Order matching logic
│   │   ├── execution_simulator.rs # Realistic execution
│   │   ├── simulation_engine.rs # Engine orchestrator
│   │   └── adapter.rs           # Integration adapter
│   ├── backtesting/
│   │   ├── engine.rs            # Backtesting engine
│   │   ├── vectorized.rs        # Vectorized operations
│   │   └── analytics.rs         # Performance metrics
│   ├── optimization/
│   │   ├── mod.rs               # Parameter optimization
│   │   └── grid_optimizer.rs    # Grid strategy optimizer
│   ├── db/
│   │   ├── strategy.rs          # Strategy database
│   │   └── trade.rs             # Trade history
│   ├── clients/
│   │   ├── kraken_api.rs        # Kraken REST API
│   │   └── kraken_ws.rs         # Kraken WebSocket
│   ├── config.rs                # Configuration types
│   ├── error.rs                 # Error handling
│   ├── validation.rs            # Pre-flight validation
│   └── progress.rs              # Progress bars
├── logs/
│   ├── portfolio/               # 🆕 Portfolio state logs (CSV)
│   └── trades/                  # 🆕 Trade execution logs (CSV)
├── config.toml                  # Your configuration
├── data/
│   └── grid_bot.db             # SQLite database
└── strategies/                  # Generated strategies
```

## Requirements

- Rust 1.70+ (2021 edition)
- Cargo
- SQLite (bundled)

## Dependencies

```toml
clap = "4.0"              # CLI framework
tokio = "1.0"             # Async runtime
polars = "0.33"           # Data processing
rusqlite = "0.31"         # SQLite database
indicatif = "0.17"        # Progress bars
reqwest = "0.11"          # HTTP client
serde = "1.0"             # Serialization
```

## License

MIT License - See LICENSE file for details

## Contributing

Contributions welcome! Please open an issue or PR.

## Support

- GitHub Issues: [Report bugs or request features](https://github.com/cryptomegabyte/grid/issues)
- Documentation: See `docs/` folder for detailed guides

## Disclaimer

This software is for educational purposes only. Cryptocurrency trading carries risk. Always test strategies thoroughly before risking real capital.
