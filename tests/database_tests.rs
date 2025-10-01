// Integration tests for database operations

mod common;

use grid_trading_bot::{Database, Strategy, StrategyService};
use common::create_temp_db_dir;

#[test]
fn test_database_creation() {
    let (_temp_dir, db_path) = create_temp_db_dir();
    
    let db = Database::new(&db_path);
    assert!(db.is_ok(), "Database creation should succeed");
}

#[test]
fn test_strategy_crud_operations() {
    let (_temp_dir, db_path) = create_temp_db_dir();
    let db = Database::new(&db_path).expect("Failed to create database");
    
    // Create strategy service
    let strategy_service = StrategyService::new(db, "strategies".to_string());
    strategy_service.init(false).expect("Failed to init service");
    
    // Test strategy creation
    let strategy = Strategy::new(
        "XRPGBP".to_string(),
        "Test Strategy".to_string(),
        10,
        0.02,
        0.55,
        0.45,
        1000.0,
    );
    
    // Insert strategy
    let strategy_id = strategy_service.save_strategy(&strategy)
        .expect("Failed to create strategy");
    assert!(strategy_id > 0, "Strategy ID should be positive");
    
    // Read strategy
    let retrieved = strategy_service.find_by_pair("XRPGBP")
        .expect("Failed to retrieve strategy");
    assert!(retrieved.is_some(), "Strategy should exist");
    
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "Test Strategy");
    assert_eq!(retrieved.pair, "XRPGBP");
    assert_eq!(retrieved.grid_levels, 10);
    
    // Update strategy
    let mut updated_strategy = retrieved.clone();
    updated_strategy.grid_levels = 15;
    updated_strategy.is_active = false;
    
    strategy_service.save_strategy(&updated_strategy)
        .expect("Failed to update strategy");
    
    let retrieved_updated = strategy_service.find_by_pair("XRPGBP")
        .expect("Failed to retrieve updated strategy")
        .unwrap();
    
    assert_eq!(retrieved_updated.grid_levels, 15);
    assert_eq!(retrieved_updated.is_active, false);
    
    // List strategies
    let all_strategies = strategy_service.list_all()
        .expect("Failed to list strategies");
    assert_eq!(all_strategies.len(), 1);
    
    // Delete strategy
    strategy_service.delete_strategy(strategy_id)
        .expect("Failed to delete strategy");
    
    let deleted = strategy_service.find_by_pair("XRPGBP")
        .expect("Failed to check deleted strategy");
    assert!(deleted.is_none(), "Strategy should be deleted");
}

#[test]
fn test_active_strategies() {
    let (_temp_dir, db_path) = create_temp_db_dir();
    let db = Database::new(&db_path).expect("Failed to create database");
    let strategy_service = StrategyService::new(db, "strategies".to_string());
    strategy_service.init(false).expect("Failed to init");
    
    // Create multiple strategies
    for i in 0..5 {
        let mut strategy = Strategy::new(
            format!("PAIR{}GBP", i),
            format!("Strategy {}", i),
            10,
            0.02,
            0.55,
            0.45,
            1000.0,
        );
        strategy.is_active = i % 2 == 0; // Only even-indexed strategies are active
        
        strategy_service.save_strategy(&strategy)
            .expect("Failed to create strategy");
    }
    
    // Get active strategies
    let active = strategy_service.list_active()
        .expect("Failed to get active strategies");
    
    assert_eq!(active.len(), 3, "Should have 3 active strategies (0, 2, 4)");
    
    for strategy in active {
        assert!(strategy.is_active, "All retrieved strategies should be active");
    }
}

#[test]
fn test_strategy_count() {
    let (_temp_dir, db_path) = create_temp_db_dir();
    let db = Database::new(&db_path).expect("Failed to create database");
    let strategy_service = StrategyService::new(db, "strategies".to_string());
    strategy_service.init(false).expect("Failed to init");
    
    // Initially no strategies
    let count = strategy_service.count().expect("Failed to count");
    assert_eq!(count, 0);
    
    // Add strategies
    for i in 0..3 {
        let strategy = Strategy::new(
            format!("PAIR{}GBP", i),
            format!("Strategy {}", i),
            10,
            0.02,
            0.55,
            0.45,
            1000.0,
        );
        strategy_service.save_strategy(&strategy).expect("Failed to save");
    }
    
    let count = strategy_service.count().expect("Failed to count");
    assert_eq!(count, 3);
}

#[test]
fn test_strategy_deactivation() {
    let (_temp_dir, db_path) = create_temp_db_dir();
    let db = Database::new(&db_path).expect("Failed to create database");
    let strategy_service = StrategyService::new(db, "strategies".to_string());
    strategy_service.init(false).expect("Failed to init");
    
    // Create active strategy
    let strategy = Strategy::new(
        "XRPGBP".to_string(),
        "Deactivation Test".to_string(),
        10,
        0.02,
        0.55,
        0.45,
        1000.0,
    );
    
    let id = strategy_service.save_strategy(&strategy)
        .expect("Failed to save");
    
    // Verify it's active
    let active = strategy_service.list_active().expect("Failed to list");
    assert_eq!(active.len(), 1);
    
    // Deactivate
    strategy_service.deactivate_strategy(id)
        .expect("Failed to deactivate");
    
    // Should have no active strategies
    let active = strategy_service.list_active().expect("Failed to list");
    assert_eq!(active.len(), 0);
}

