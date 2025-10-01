// Common test utilities and helpers

use grid_trading_bot::{Config, TradingConfig, MarketConfig, LoggingConfig};
use tempfile::TempDir;
use std::path::PathBuf;

/// Create a test configuration with sensible defaults
pub fn create_test_config() -> Config {
    Config {
        trading: TradingConfig {
            kraken_ws_url: "wss://ws.kraken.com".to_string(),
            trading_pair: "XRPGBP".to_string(),
            grid_levels: 5,
            grid_spacing: 0.02,
            min_price_change: 0.001,
        },
        market: MarketConfig {
            trend_threshold: 0.005,
            volatility_threshold: 0.02,
            price_history_size: 10,
        },
        logging: LoggingConfig {
            enable_price_logging: false,
            enable_signal_logging: false,
            enable_state_change_logging: false,
        },
    }
}

/// Create a temporary directory for test databases
pub fn create_temp_db_dir() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");
    (temp_dir, db_path)
}

/// Generate test price data for backtesting
pub fn generate_test_prices(base_price: f64, count: usize, volatility: f64) -> Vec<f64> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut prices = Vec::with_capacity(count);
    let mut current_price = base_price;
    
    for _ in 0..count {
        let change_pct = rng.gen_range(-volatility..volatility);
        current_price *= 1.0 + change_pct;
        prices.push(current_price);
    }
    
    prices
}

/// Generate test timestamps
pub fn generate_test_timestamps(count: usize, interval_minutes: i64) -> Vec<chrono::DateTime<chrono::Utc>> {
    use chrono::{Utc, Duration};
    
    let start = Utc::now() - Duration::days(30);
    (0..count)
        .map(|i| start + Duration::minutes(i as i64 * interval_minutes))
        .collect()
}
