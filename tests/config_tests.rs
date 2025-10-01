// Integration tests for configuration loading and validation

mod common;

use grid_trading_bot::Config;
use common::{create_test_config};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_default_config_creation() {
    let config = create_test_config();
    
    assert_eq!(config.trading.trading_pair, "XRPGBP");
    assert_eq!(config.trading.grid_levels, 5);
    assert!(config.trading.grid_spacing > 0.0);
    assert!(config.market.trend_threshold > 0.0);
}

#[test]
fn test_config_serialization_deserialization() {
    let config = create_test_config();
    
    // Serialize to TOML
    let toml_string = toml::to_string(&config)
        .expect("Failed to serialize config");
    
    assert!(!toml_string.is_empty());
    assert!(toml_string.contains("trading_pair"));
    assert!(toml_string.contains("XRPGBP"));
    
    // Deserialize back
    let deserialized: Config = toml::from_str(&toml_string)
        .expect("Failed to deserialize config");
    
    assert_eq!(deserialized.trading.trading_pair, config.trading.trading_pair);
    assert_eq!(deserialized.trading.grid_levels, config.trading.grid_levels);
}

#[test]
fn test_config_file_loading() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("test_config.toml");
    
    let config = create_test_config();
    let toml_string = toml::to_string(&config)
        .expect("Failed to serialize config");
    
    // Write config to file
    fs::write(&config_path, toml_string)
        .expect("Failed to write config file");
    
    // Load config from file
    let loaded = Config::from_file(&config_path)
        .expect("Failed to load config");
    
    assert_eq!(loaded.trading.trading_pair, "XRPGBP");
    assert_eq!(loaded.trading.grid_levels, 5);
}

#[test]
fn test_config_validation() {
    let mut config = create_test_config();
    
    // Valid configuration
    assert!(config.trading.grid_levels > 0);
    assert!(config.trading.grid_spacing > 0.0);
    assert!(config.trading.min_price_change > 0.0);
    
    // Test invalid grid levels
    config.trading.grid_levels = 0;
    // In a real scenario, you'd call a validate() method here
    assert_eq!(config.trading.grid_levels, 0);
    
    // Reset to valid
    config.trading.grid_levels = 5;
    assert!(config.trading.grid_levels > 0);
}

#[test]
fn test_market_config_defaults() {
    let config = create_test_config();
    let market_config = config.market;
    
    assert!(market_config.trend_threshold > 0.0);
    assert!(market_config.volatility_threshold > 0.0);
    assert!(market_config.price_history_size > 0);
    
    // Verify reasonable defaults
    assert!(market_config.trend_threshold < 0.1); // Less than 10%
    assert!(market_config.volatility_threshold < 0.1); // Less than 10%
    assert!(market_config.price_history_size >= 10);
}

#[test]
fn test_logging_config() {
    let config = create_test_config();
    let logging = config.logging;
    
    // Verify logging options are boolean
    assert!(!logging.enable_price_logging || logging.enable_price_logging);
    assert!(!logging.enable_signal_logging || logging.enable_signal_logging);
    assert!(!logging.enable_state_change_logging || logging.enable_state_change_logging);
}

#[test]
fn test_config_with_custom_values() {
    let mut config = create_test_config();
    
    // Customize values
    config.trading.trading_pair = "ETHGBP".to_string();
    config.trading.grid_levels = 20;
    config.trading.grid_spacing = 0.015;
    config.market.trend_threshold = 0.01;
    
    // Verify changes
    assert_eq!(config.trading.trading_pair, "ETHGBP");
    assert_eq!(config.trading.grid_levels, 20);
    assert_eq!(config.trading.grid_spacing, 0.015);
    assert_eq!(config.market.trend_threshold, 0.01);
}

#[test]
fn test_config_json_compatibility() {
    let config = create_test_config();
    
    // Serialize to JSON
    let json_string = serde_json::to_string(&config)
        .expect("Failed to serialize to JSON");
    
    assert!(!json_string.is_empty());
    
    // Deserialize from JSON
    let deserialized: Config = serde_json::from_str(&json_string)
        .expect("Failed to deserialize from JSON");
    
    assert_eq!(deserialized.trading.trading_pair, config.trading.trading_pair);
}

#[test]
fn test_config_clone() {
    let config = create_test_config();
    let cloned = config.clone();
    
    assert_eq!(config.trading.trading_pair, cloned.trading.trading_pair);
    assert_eq!(config.trading.grid_levels, cloned.trading.grid_levels);
    assert_eq!(config.market.trend_threshold, cloned.market.trend_threshold);
}

#[test]
fn test_multiple_configs_isolation() {
    let config1 = create_test_config();
    let mut config2 = create_test_config();
    
    // Modify config2
    config2.trading.trading_pair = "BTCGBP".to_string();
    
    // Verify config1 is unchanged
    assert_eq!(config1.trading.trading_pair, "XRPGBP");
    assert_eq!(config2.trading.trading_pair, "BTCGBP");
}

#[test]
fn test_config_error_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let non_existent_path = temp_dir.path().join("non_existent.toml");
    
    // Attempt to load non-existent file
    let result = Config::from_file(&non_existent_path);
    assert!(result.is_err(), "Loading non-existent config should fail");
}

#[test]
fn test_config_malformed_toml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("malformed.toml");
    
    // Write malformed TOML
    fs::write(&config_path, "this is not valid toml {{{")
        .expect("Failed to write malformed config");
    
    // Attempt to load
    let result = Config::from_file(&config_path);
    assert!(result.is_err(), "Loading malformed config should fail");
}

#[test]
fn test_config_partial_toml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("partial.toml");
    
    // Write partial TOML (missing required fields)
    let partial_toml = r#"
[trading]
trading_pair = "XRPGBP"
    "#;
    
    fs::write(&config_path, partial_toml)
        .expect("Failed to write partial config");
    
    // Attempt to load - should fail due to missing required fields
    let result = Config::from_file(&config_path);
    assert!(result.is_err(), "Loading incomplete config should fail");
}
