.PHONY: help build backtest optimize trade test clean

# Default target - show help
help:
	@echo "🚀 Grid Trading Bot - Simple Commands"
	@echo ""
	@echo "Essential Commands:"
	@echo "  make backtest     - Run autonomous optimization (generates optimized strategies)"
	@echo "  make trade        - Start LIVE trading with real market data (indefinite)"
	@echo "  make trade-market - LIVE trade for 8 hours with real Kraken feeds"
	@echo "  make trade-demo   - 5-minute LIVE demo with real data"
	@echo "  make test         - Run all tests"
	@echo "  make clean        - Clean everything" 
	@echo ""
	@echo "Advanced:"
	@echo "  make optimize PAIR=ADAGBP    - Optimize specific pair"
	@echo "  make trade-hours HOURS=6     - Trade for custom hours"
	@echo "  make full-session HOURS=8    - Complete workflow: optimize + trade"
	@echo ""
	@echo "📁 All strategies saved to: strategies/"

# Build the project
build:
	cargo build

# Best backtest command - runs autonomous optimization with optimal settings
backtest:
	@echo "🎯 Running autonomous optimization (generates optimized strategies)..."
	cargo run --bin backtest -- optimize-gbp --limit 10 --iterations 20 --strategy random-search

# Optimize specific pair (usage: make optimize PAIR=ADAGBP)
optimize:
	@echo "🔍 Optimizing $(PAIR)..."
	cargo run --bin backtest -- optimize-pair --pair $(PAIR) --iterations 10

# Start LIVE trading with real market data (indefinite)
trade:
	@echo "� Starting LIVE trading with real Kraken market data..."
	@echo "🎯 Using adaptive grids based on volatility, support/resistance"
	RUST_LOG=info cargo run --bin trade start

# Trade for a specific number of hours with real data
trade-hours:
	@echo "� Starting LIVE trading with real market data for $(HOURS) hours..."
	RUST_LOG=info cargo run --bin trade start --hours $(HOURS)

# Trade for market hours (8 hours) with real data
trade-market:
	@echo "� Starting LIVE trading with real Kraken feeds for market hours (8h)..."
	@echo "🎯 Using intelligent grid placement based on market conditions"
	RUST_LOG=info cargo run --bin trade start --hours 8

# Demo LIVE trading for 5 minutes with real market data
trade-demo:
	@echo "📡 Running 5-minute LIVE trading demo with real Kraken data..."
	@echo "🎯 Testing adaptive grid intelligence"
	RUST_LOG=info cargo run --bin trade start --minutes 5

# Complete workflow: backtest optimization + live trading (usage: make full-session HOURS=8)
full-session:
	@echo "🎯 Starting complete trading session for $(HOURS) hours..."
	@echo "📊 Phase 1: Running autonomous optimization..."
	cargo run --bin backtest -- optimize-gbp --limit 10 --iterations 20 --strategy random-search
	@echo "✅ Optimization complete! Starting live trading..."
	@echo "📡 Phase 2: LIVE trading with optimized strategies for $(HOURS) hours"
	@echo "🚀 Using real Kraken market data with adaptive grids"
	RUST_LOG=info cargo run --bin trade start --hours $(HOURS)

# Run tests
test:
	cargo test

# Clean everything
clean:
	cargo clean
	rm -rf strategies/ *.md
	@echo "✅ Cleaned all artifacts"