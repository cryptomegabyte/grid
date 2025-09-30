.PHONY: build run clean test check fmt clippy help

# Default target
help:
	@echo "Available commands:"
	@echo "  build    - Build the project"
	@echo "  run      - Run the grid trading bot"
	@echo "  clean    - Clean build artifacts"
	@echo "  test     - Run tests"
	@echo "  check    - Check code without building"
	@echo "  fmt      - Format code"
	@echo "  clippy   - Run clippy linter"
	@echo "  dev      - Run in development mode with logs"
	@echo "  release  - Build optimized release version"

# Build the project
build:
	cargo build

# Run the trading bot
run:
	cargo run

# Run in development mode with detailed output
dev:
	RUST_LOG=debug cargo run

# Build optimized release version
release:
	cargo build --release

# Run the release version
run-release:
	cargo run --release

# Clean build artifacts
clean:
	cargo clean

# Run tests
test:
	cargo test

# Run tests with output
test-verbose:
	cargo test -- --nocapture

# Run only unit tests (from main.rs)
test-unit:
	cargo test --bin grid-trading-bot

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

# Quick development cycle: format, check, test
dev-check: fmt clippy test

# Show project info
info:
	@echo "Grid Trading Bot - Rust Project"
	@echo "Trading Pair: XRP/GBP"
	@echo "WebSocket: Kraken Public Feed"
	@cargo --version
	@rustc --version