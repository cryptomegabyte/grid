.PHONY: help build backtest optimize trade test clean

# Default target - show help
help:
	@echo "🚀 Grid Trading Bot - Simple Commands"
	@echo ""
	@echo "Essential Commands:"
	@echo "  make backtest   - Run autonomous GBP optimization (best strategy)"
	@echo "  make trade      - Start live trading"
	@echo "  make test       - Run all tests"
	@echo "  make clean      - Clean everything"
	@echo ""
	@echo "Advanced:"
	@echo "  make optimize PAIR=ADAGBP  - Optimize specific pair"

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

# Start live trading
trade:
	@echo "💰 Starting live trading system..."
	RUST_LOG=info cargo run --bin trade

# Run tests
test:
	cargo test

# Clean everything
clean:
	cargo clean
	rm -rf strategies/ optimized_strategies/ *.md
	@echo "✅ Cleaned all artifacts"