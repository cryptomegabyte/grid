.PHONY: build run clean test check fmt clippy help

# Default target
help:
	@echo "Available commands:"
	@echo "  build       - Build the project"
	@echo "  optimize    - Run autonomous GBP pair optimization"
	@echo "  optimize-advanced - Run advanced genetic algorithm optimization"
	@echo "  optimize-single - Optimize specific pair (use PAIR=GBPUSD)"
	@echo "  backtest    - Run backtesting system"
	@echo "  trade       - Run live trading system"
	@echo "  clean       - Clean build artifacts, strategies, optimization results, and reports"
	@echo "  test        - Run tests"
	@echo "  check       - Check code without building"
	@echo "  fmt         - Format code"
	@echo "  clippy      - Run clippy linter"
	@echo "  dev         - Run in development mode with logs"
	@echo "  release     - Build optimized release version"

# Build the project
build:
	cargo build

# Autonomous optimization commands
optimize:
	cargo run --bin backtest -- optimize-gbp

optimize-advanced:
	cargo run --bin backtest -- optimize-gbp --strategy genetic-algorithm --timeframes --risk-optimization --iterations 200

optimize-single:
	cargo run --bin backtest -- optimize-pair --pair $(PAIR) --comprehensive

optimize-report:
	cargo run --bin backtest -- optimize-gbp --strategy genetic-algorithm --report --iterations 500

# Run backtesting system
backtest:
	cargo run --bin backtest

# Run live trading system
trade:
	cargo run --bin trade

# Run backtesting with detailed logs
backtest-dev:
	RUST_LOG=debug cargo run --bin backtest

# Run trading with detailed logs
trade-dev:
	RUST_LOG=debug cargo run --bin trade

# Build optimized release version
release:
	cargo build --release

# Run backtest in release mode
backtest-release:
	cargo run --release --bin backtest

# Run trade in release mode
trade-release:
	cargo run --release --bin trade

# Clean build artifacts and generated files
clean:
	cargo clean
	rm -rf strategies/
	rm -rf optimized_strategies/
	rm -f portfolio_analysis.md
	rm -f optimization_report.md
	rm -f *_optimization_analysis.md
	@echo "Cleaned build artifacts, strategies, optimization results, and reports"

# Run tests
test:
	cargo test

# Run tests with output
test-verbose:
	cargo test -- --nocapture

# Run library tests
test-lib:
	cargo test --lib

# Run binary tests
test-bin:
	cargo test --bins

# Run only e2e tests  
test-e2e:
	cargo test --test e2e_tests

# Run e2e tests with output
test-e2e-verbose:
	cargo test --test e2e_tests -- --nocapture

# Check code without building
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Run clippy linter
clippy:
	cargo clippy -- -D warnings

# Install dependencies and build
install:
	cargo fetch
	cargo build

# Run backtest demo
demo-backtest:
	cargo run --bin backtest demo

# Run trade demo
demo-trade:
	cargo run --bin trade demo

# List available pairs
list-pairs:
	cargo run --bin backtest list

# Quick development cycle: format, check, test
dev-check: fmt clippy test

# Show project info
info:
	@echo "Grid Trading Bot - Professional Rust Project"
	@echo "Binaries: backtest, trade"
	@echo "Structure: core/, clients/, backtesting/"
	@echo "API: Kraken WebSocket & REST"
	@cargo --version
	@rustc --version