.PHONY: help backtest trade trade-sim trade-live test test-sim clean build

help:
	@echo "🚀 Grid Trading Bot - Quick Commands"
	@echo ""
	@echo "Trading Commands:"
	@echo "  make trade-sim      - Paper trading with simulation engine (30 min)"
	@echo "  make trade-live     - Live trading without simulation (use with caution!)"
	@echo "  make trade          - Interactive: choose mode and duration"
	@echo ""
	@echo "Development Commands:"
	@echo "  make backtest       - Run backtesting optimization"
	@echo "  make test           - Run all tests"
	@echo "  make test-sim       - Run simulation engine tests only"
	@echo "  make build          - Build all binaries"
	@echo "  make clean          - Clean build artifacts"

backtest:
	@echo "🎯 Running backtesting optimization..."
	@cargo run --bin grid-bot -- optimize all --limit 10 --iterations 20

trade-sim:
	@echo "� Starting Paper Trading with Simulation Engine"
	@echo "� Capital: £500 | ⏱️  Duration: 30 minutes"
	@echo "📊 Using strategies from: strategies/"
	@echo ""
	@cargo run --bin trade start --simulate --capital 500 --strategies-dir strategies --minutes 30

trade-live:
	@echo "⚠️  WARNING: LIVE TRADING MODE (No Simulation)"
	@echo "💰 Capital: £500 | ⏱️  Duration: 30 minutes"
	@echo "� Using strategies from: strategies/"
	@echo ""
	@read -p "Press Enter to continue or Ctrl+C to cancel..."
	@cargo run --bin trade start --capital 500 --strategies-dir strategies --minutes 30

trade:
	@echo "🎯 Grid Trading Bot - Interactive Mode"
	@echo ""
	@echo "Select trading mode:"
	@echo "  1) Paper Trading (with simulation engine) - SAFE ✅"
	@echo "  2) Live Trading (without simulation) - USE WITH CAUTION ⚠️"
	@read -p "Enter choice [1-2]: " mode; \
	read -p "Enter capital (GBP): " capital; \
	read -p "Enter duration (minutes): " minutes; \
	if [ "$$mode" = "1" ]; then \
		echo "🎮 Starting paper trading with simulation engine..."; \
		cargo run --bin trade start --simulate --capital $$capital --strategies-dir strategies --minutes $$minutes; \
	else \
		echo "⚠️  Starting LIVE trading..."; \
		cargo run --bin trade start --capital $$capital --strategies-dir strategies --minutes $$minutes; \
	fi

build:
	@echo "🔨 Building all binaries..."
	@cargo build --release
	@echo "✅ Build complete! Binaries in target/release/"

test:
	@echo "🧪 Running all tests..."
	@cargo test

test-sim:
	@echo "🎮 Running simulation engine tests..."
	@cargo test --lib simulation

clean:
	@cargo clean
	@echo "✅ Cleaned"
