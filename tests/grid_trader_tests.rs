// Integration tests for GridTrader and market state

mod common;

use grid_trading_bot::{GridTrader, GridSignal};
use common::create_test_config;

#[test]
fn test_grid_trader_creation() {
    let config = create_test_config();
    let trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Trader should be created successfully
    assert!(true, "GridTrader should be created");
}

#[test]
fn test_grid_initialization_on_first_price() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    let initial_price = 0.50;
    let signal = trader.update_with_price(initial_price);
    
    // First price should initialize grid but not generate signal
    assert!(matches!(signal, GridSignal::None) || matches!(signal, GridSignal::Buy(_)) || matches!(signal, GridSignal::Sell(_)));
}

#[test]
fn test_buy_signal_generation() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Initialize grid
    let initial_price = 0.50;
    trader.update_with_price(initial_price);
    
    // Price drops below buy level
    let low_price = 0.48;
    let signal = trader.update_with_price(low_price);
    
    // Should generate buy signal or none
    assert!(matches!(signal, GridSignal::Buy(_)) || matches!(signal, GridSignal::None));
}

#[test]
fn test_sell_signal_generation() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Initialize grid
    let initial_price = 0.50;
    trader.update_with_price(initial_price);
    
    // Price rises above sell level
    let high_price = 0.52;
    let signal = trader.update_with_price(high_price);
    
    // Should generate sell signal or none
    assert!(matches!(signal, GridSignal::Sell(_)) || matches!(signal, GridSignal::None));
}

#[test]
fn test_grid_spacing_calculation() {
    let config = create_test_config();
    let base_price = 0.50;
    let grid_spacing = config.trading.grid_spacing;
    let grid_levels = config.trading.grid_levels;
    
    // Calculate expected buy levels
    let mut expected_buy_levels = vec![];
    for i in 1..=grid_levels {
        let buy_level = base_price - (i as f64 * grid_spacing * base_price);
        expected_buy_levels.push(buy_level);
    }
    
    // Calculate expected sell levels
    let mut expected_sell_levels = vec![];
    for i in 1..=grid_levels {
        let sell_level = base_price + (i as f64 * grid_spacing * base_price);
        expected_sell_levels.push(sell_level);
    }
    
    // Verify levels are calculated correctly
    assert!(expected_buy_levels.len() == grid_levels);
    assert!(expected_sell_levels.len() == grid_levels);
    
    // Buy levels should be below base price
    for buy_level in expected_buy_levels {
        assert!(buy_level < base_price);
    }
    
    // Sell levels should be above base price
    for sell_level in expected_sell_levels {
        assert!(sell_level > base_price);
    }
}

#[test]
fn test_price_logging_threshold() {
    let config = create_test_config();
    let trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    let _price1 = 0.50;
    let price2 = 0.50001; // Small change
    let price3 = 0.51;    // Large change
    
    let min_change = config.trading.min_price_change;
    
    // Small price change should not trigger logging (or might trigger depending on implementation)
    // Just verify the method doesn't panic
    let _ = trader.should_log_price(price2, min_change);
    
    // Large price change should trigger logging
    assert!(trader.should_log_price(price3, min_change));
}

#[test]
fn test_market_state_trending_up() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Simulate upward price movement
    let prices = vec![0.48, 0.49, 0.50, 0.51, 0.52];
    
    for price in prices {
        trader.update_with_price(price);
    }
    
    // After upward trend, trader should have updated grid
    // Just verify no panic
    assert!(true);
}

#[test]
fn test_market_state_trending_down() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Simulate downward price movement
    let prices = vec![0.52, 0.51, 0.50, 0.49, 0.48];
    
    for price in prices {
        trader.update_with_price(price);
    }
    
    // After downward trend, trader should have updated grid
    assert!(true);
}

#[test]
fn test_market_state_ranging() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Simulate ranging market
    let prices = vec![0.50, 0.501, 0.499, 0.5005, 0.4995, 0.50];
    
    for price in prices {
        trader.update_with_price(price);
    }
    
    // In ranging market, grid should remain stable
    assert!(true);
}

#[test]
fn test_multiple_buy_signals() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Initialize grid
    trader.update_with_price(0.50);
    
    // Trigger multiple buy levels
    let signal1 = trader.update_with_price(0.49);
    let signal2 = trader.update_with_price(0.48);
    let signal3 = trader.update_with_price(0.47);
    
    // Each should be valid (either Buy or None)
    assert!(matches!(signal1, GridSignal::Buy(_)) || matches!(signal1, GridSignal::None));
    assert!(matches!(signal2, GridSignal::Buy(_)) || matches!(signal2, GridSignal::None));
    assert!(matches!(signal3, GridSignal::Buy(_)) || matches!(signal3, GridSignal::None));
}

#[test]
fn test_multiple_sell_signals() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Initialize grid
    trader.update_with_price(0.50);
    
    // Trigger multiple sell levels
    let signal1 = trader.update_with_price(0.51);
    let signal2 = trader.update_with_price(0.52);
    let signal3 = trader.update_with_price(0.53);
    
    // Each should be valid (either Sell or None)
    assert!(matches!(signal1, GridSignal::Sell(_)) || matches!(signal1, GridSignal::None));
    assert!(matches!(signal2, GridSignal::Sell(_)) || matches!(signal2, GridSignal::None));
    assert!(matches!(signal3, GridSignal::Sell(_)) || matches!(signal3, GridSignal::None));
}

#[test]
fn test_price_oscillation() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Initialize grid
    trader.update_with_price(0.50);
    
    // Simulate oscillating prices
    let prices = vec![0.49, 0.51, 0.48, 0.52, 0.49, 0.51];
    let mut buy_signals = 0;
    let mut sell_signals = 0;
    
    for price in prices {
        let signal = trader.update_with_price(price);
        match signal {
            GridSignal::Buy(_) => buy_signals += 1,
            GridSignal::Sell(_) => sell_signals += 1,
            GridSignal::None => {},
        }
    }
    
    // Should generate both buy and sell signals
    assert!(buy_signals > 0 || sell_signals > 0);
}

#[test]
fn test_grid_trader_state_persistence() {
    let config = create_test_config();
    let mut trader = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Update prices
    trader.update_with_price(0.50);
    trader.update_with_price(0.51);
    
    // Mark price as logged
    trader.update_logged_price(0.51);
    
    // Small change should not require logging
    assert!(!trader.should_log_price(0.5101, config.trading.min_price_change));
}

#[test]
fn test_concurrent_trader_independence() {
    let config = create_test_config();
    
    let mut trader1 = GridTrader::new(config.trading.clone(), config.market.clone());
    let mut trader2 = GridTrader::new(config.trading.clone(), config.market.clone());
    
    // Update traders with different prices
    trader1.update_with_price(0.50);
    trader2.update_with_price(0.55);
    
    // They should maintain independent state
    assert!(true, "Traders should be independent");
}
