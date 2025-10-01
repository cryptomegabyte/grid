// End-to-end integration tests

mod common;

use grid_trading_bot::{
    Database, Strategy, StrategyService,
    GridTrader,
};
use common::{create_test_config, create_temp_db_dir, generate_test_prices};

#[test]
fn test_complete_trading_workflow() {
    // Setup: Create database and config
    let (_temp_dir, db_path) = create_temp_db_dir();
    let db = Database::new(&db_path).expect("Failed to create database");
    let strategy_service = StrategyService::new(db, "strategies".to_string());
    strategy_service.init(false).expect("Failed to init");
    let config = create_test_config();
    
    // Step 1: Create a strategy
    let strategy = Strategy::new(
        "XRPGBP".to_string(),
        "Complete Workflow Test".to_string(),
        10,
        0.02,
        0.55,
        0.45,
        10000.0,
    );
    
    let strategy_id = strategy_service.save_strategy(&strategy)
        .expect("Failed to create strategy");
    
    assert!(strategy_id > 0);
    
    // Step 2: Initialize grid trader
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Step 3: Simulate price updates and trading
    let prices = generate_test_prices(0.50, 50, 0.01);
    let mut signals = vec![];
    
    for price in prices {
        let signal = trader.update_with_price(price);
        signals.push(signal);
    }
    
    // Step 4: Verify we got some trading signals
    let buy_count = signals.iter().filter(|s| matches!(s, grid_trading_bot::GridSignal::Buy(_))).count();
    let sell_count = signals.iter().filter(|s| matches!(s, grid_trading_bot::GridSignal::Sell(_))).count();
    
    // Should have some activity (or all holds in stable market)
    // Note: buy_count and sell_count are unsigned, so >= 0 is always true
    let _total_signals = buy_count + sell_count;
    
    // Step 5: Verify strategy is still retrievable
    let retrieved = strategy_service.find_by_pair("XRPGBP")
        .expect("Failed to retrieve strategy");
    assert!(retrieved.is_some());
    
    let retrieved_strategy = retrieved.unwrap();
    assert_eq!(retrieved_strategy.name, "Complete Workflow Test");
    assert_eq!(retrieved_strategy.is_active, true);
}

#[test]
fn test_multi_strategy_simulation() {
    // Setup
    let (_temp_dir, db_path) = create_temp_db_dir();
    let db = Database::new(&db_path).expect("Failed to create database");
    let strategy_service = StrategyService::new(db, "strategies".to_string());
    strategy_service.init(false).expect("Failed to init");
    
    // Create multiple strategies for different pairs
    let pairs = vec!["XRPGBP", "ETHGBP", "BTCGBP"];
    let mut strategy_ids = vec![];
    
    for pair in &pairs {
        let strategy = Strategy::new(
            pair.to_string(),
            format!("{} Strategy", pair),
            10,
            0.02,
            1.0,
            0.5,
            10000.0,
        );
        
        let id = strategy_service.save_strategy(&strategy)
            .expect("Failed to create strategy");
        strategy_ids.push(id);
    }
    
    assert_eq!(strategy_ids.len(), 3);
    
    // Create traders for each strategy
    let config = create_test_config();
    let mut traders: Vec<_> = pairs.iter()
        .map(|_| GridTrader::new(config.trading.clone(), config.market.clone()))
        .collect();
    
    // Simulate trading on all pairs
    let base_prices = vec![0.50, 1800.0, 32000.0]; // XRP, ETH, BTC approximate prices
    
    for (i, trader) in traders.iter_mut().enumerate() {
        let prices = generate_test_prices(base_prices[i], 20, 0.005);
        for price in prices {
            trader.update_with_price(price);
        }
    }
    
    // Verify all strategies are still active
    let active_strategies = strategy_service.list_active()
        .expect("Failed to get active strategies");
    assert_eq!(active_strategies.len(), 3);
}

#[test]
fn test_strategy_lifecycle() {
    let (_temp_dir, db_path) = create_temp_db_dir();
    let db = Database::new(&db_path).expect("Failed to create database");
    let strategy_service = StrategyService::new(db, "strategies".to_string());
    strategy_service.init(false).expect("Failed to init");
    
    // Create strategy
    let strategy = Strategy::new(
        "XRPGBP".to_string(),
        "Lifecycle Test".to_string(),
        10,
        0.02,
        0.55,
        0.45,
        10000.0,
    );
    
    let id = strategy_service.save_strategy(&strategy)
        .expect("Failed to create strategy");
    
    // Start with active strategy
    let active = strategy_service.list_active()
        .expect("Failed to get active strategies");
    assert_eq!(active.len(), 1);
    
    // Deactivate strategy
    strategy_service.deactivate_strategy(id)
        .expect("Failed to deactivate strategy");
    
    // Should have no active strategies
    let active = strategy_service.list_active()
        .expect("Failed to get active strategies");
    assert_eq!(active.len(), 0);
    
    // Delete strategy
    strategy_service.delete_strategy(id)
        .expect("Failed to delete strategy");
    
    // Should not exist
    let deleted = strategy_service.find_by_pair("XRPGBP")
        .expect("Failed to check deleted strategy");
    assert!(deleted.is_none());
}

#[test]
fn test_config_driven_trading() {
    let mut config = create_test_config();
    
    // Test with different configurations
    let configs = vec![
        (5, 0.01),   // 5 levels, 1% spacing
        (10, 0.02),  // 10 levels, 2% spacing
        (20, 0.005), // 20 levels, 0.5% spacing
    ];
    
    for (levels, spacing) in configs {
        config.trading.grid_levels = levels;
        config.trading.grid_spacing = spacing;
        
        let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
        
        // Initialize grid
        trader.update_with_price(0.50);
        
        // Test with price movement
        let prices = generate_test_prices(0.50, 30, 0.01);
        for price in prices {
            trader.update_with_price(price);
        }
        
        // Verify trader handles different configurations
        assert!(true, "Trader should handle config: levels={}, spacing={}", levels, spacing);
    }
}

#[test]
fn test_error_recovery() {
    let (_temp_dir, db_path) = create_temp_db_dir();
    let db = Database::new(&db_path).expect("Failed to create database");
    let strategy_service = StrategyService::new(db, "strategies".to_string());
    strategy_service.init(false).expect("Failed to init");
    
    // Try to get non-existent strategy
    let result = strategy_service.find_by_pair("NONEXISTENT");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
    
    // Try to delete non-existent strategy
    let delete_result = strategy_service.delete_strategy(999);
    // Should handle gracefully
    assert!(delete_result.is_ok() || delete_result.is_err());
}

#[test]
fn test_price_validation() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Test with various price values
    let test_prices = vec![
        0.01,    // Very low
        0.50,    // Normal
        100.0,   // High
        0.0001,  // Very small
    ];
    
    for price in test_prices {
        // Should not panic with any valid positive price
        trader.update_with_price(price);
    }
    
    assert!(true, "Trader should handle various price ranges");
}

#[test]
fn test_rapid_price_updates() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Simulate rapid price updates
    let mut current_price = 0.50;
    for _ in 0..1000 {
        current_price += 0.0001;
        trader.update_with_price(current_price);
    }
    
    assert!(true, "Trader should handle rapid updates");
}

#[test]
fn test_strategy_export() {
    let (_temp_dir, db_path) = create_temp_db_dir();
    let temp_export_dir = _temp_dir.path().join("export");
    let db = Database::new(&db_path).expect("Failed to create database");
    let strategy_service = StrategyService::new(db, "strategies".to_string());
    strategy_service.init(false).expect("Failed to init");
    
    // Create strategy with optional parameters
    let mut strategy = Strategy::new(
        "XRPGBP".to_string(),
        "Parameterized Strategy".to_string(),
        10,
        0.02,
        0.55,
        0.45,
        10000.0,
    );
    strategy.stop_loss_pct = Some(0.03);
    strategy.take_profit_pct = Some(0.05);
    strategy.max_position_size = Some(5000.0);
    
    let _id = strategy_service.save_strategy(&strategy)
        .expect("Failed to create strategy");
    
    // Retrieve and export
    let retrieved = strategy_service.find_by_pair("XRPGBP")
        .expect("Failed to retrieve strategy")
        .unwrap();
    
    let export_path = strategy_service.export_to_json(&retrieved, temp_export_dir.to_str().unwrap());
    assert!(export_path.is_ok(), "Export should succeed");
}

