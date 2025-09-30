.PHONY: help build backtest optimize trade test clean

# Default target - show help
help:
	@echo "🚀 Grid Trading Bot - Simple Commands"
	@echo ""
	@echo "Essential Commands:"
	@echo "  make backtest     - Run autonomous GBP optimization (best strategy)"
	@echo "  make trade        - Start live trading (indefinite)"
	@echo "  make trade-market - Trade for 8 hours (market session)"
	@echo "  make trade-demo   - 5-minute demo"
	@echo "  make test         - Run all tests"
	@echo "  make clean        - Clean everything" 
	@echo ""
	@echo "Advanced:"
	@echo "  make optimize PAIR=ADAGBP    - Optimize specific pair"
	@echo "  make trade-hours HOURS=6     - Trade for custom hours"

# Build the project
build:
	cargo build

# Best backtest command - runs autonomous optimization with optimal settings
backtest:
	@echo "🎯 Running autonomous GBP optimization with best settings..."
	cargo run --bin backtest -- optimize-gbp --limit 10 --iterations 20 --strategy random-search

# Optimize specific pair (usage: make optimize PAIR=ADAGBP)
optimize:
	@echo "🔍 Optimizing $(PAIR)..."
	cargo run --bin backtest -- optimize-pair --pair $(PAIR) --iterations 10

# Start live trading simulation with all optimized strategies (indefinite)
trade:
	@echo "💰 Starting live trading simulation with optimized strategies..."
	RUST_LOG=info cargo run --bin trade start

# Trade for a specific number of hours (usage: make trade-hours HOURS=8)
trade-hours:
	@echo "💰 Starting live trading simulation for $(HOURS) hours..."
	RUST_LOG=info cargo run --bin trade start --hours $(HOURS)

# Trade for market hours (8 hours)
trade-market:
	@echo "💰 Starting live trading simulation for market hours (8h)..."
	RUST_LOG=info cargo run --bin trade start --hours 8

# Demo the trading system for 5 minutes
trade-demo:
	@echo "🎯 Running 5-minute trading simulation demo..."
	RUST_LOG=info cargo run --bin trade start --minutes 5

# Run tests
test:
	cargo test

# Clean everything
clean:
	cargo clean
	rm -rf strategies/ optimized_strategies/ *.md
	@echo "✅ Cleaned all artifacts"