.PHONY: help backtest full-workflow test clean

help:
	@echo "🚀 Grid Trading Bot"
	@echo ""
	@echo "Commands:"
	@echo "  make backtest       - Run backtesting optimization"
	@echo "  make full-workflow  - Complete: backtest + simulated trading"
	@echo "  make test           - Run all tests"
	@echo "  make clean          - Clean build artifacts"

backtest:
	@echo "🎯 Running backtesting optimization..."
	@cargo run --bin grid-bot -- backtest optimize --limit 10 --iterations 20

full-workflow:
	@echo "🎯 Complete Trading Workflow"
	@echo "📊 Phase 1: Backtesting Optimization"
	@cargo run --bin grid-bot -- backtest optimize --limit 10 --iterations 20
	@echo ""
	@echo "✅ Optimization complete!"
	@echo "📡 Phase 2: Simulated Trading (Dry-Run)"
	@cargo run --bin grid-bot -- trade start --dry-run --capital 500

test:
	@cargo test

clean:
	@cargo clean
	@echo "✅ Cleaned"
