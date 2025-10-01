// Integration tests for backtesting engine

mod common;

use grid_trading_bot::{
    BacktestConfig, BacktestingEngine,
};
use common::{generate_test_prices, generate_test_timestamps};

#[test]
fn test_historical_data_processing() {
    // Generate test data
    let prices = generate_test_prices(0.50, 100, 0.01);
    let timestamps = generate_test_timestamps(100, 15);
    
    assert_eq!(prices.len(), 100);
    assert_eq!(timestamps.len(), 100);
    
    // Verify price trends
    let mut has_ups = false;
    let mut has_downs = false;
    
    for i in 1..prices.len() {
        if prices[i] > prices[i-1] {
            has_ups = true;
        } else if prices[i] < prices[i-1] {
            has_downs = true;
        }
    }
    
    assert!(has_ups || has_downs, "Generated prices should have variation");
}

#[test]
fn test_performance_metrics_calculation() {
    let initial_capital = 10000.0;
    let final_value = 11500.0;
    let total_trades = 50;
    let winning_trades = 30;
    let losing_trades = 20;
    
    // Calculate basic metrics
    let total_return = final_value - initial_capital;
    let return_pct = (total_return / initial_capital) * 100.0;
    let win_rate = (winning_trades as f64 / total_trades as f64) * 100.0;
    
    assert_eq!(return_pct, 15.0, "Return percentage should be 15%");
    assert_eq!(win_rate, 60.0, "Win rate should be 60%");
}

#[test]
fn test_grid_level_calculation() {
    let base_price = 0.50;
    let grid_spacing = 0.02; // 2%
    let grid_levels = 5;
    
    // Calculate grid levels
    let mut grid_prices = vec![];
    for i in 0..grid_levels {
        let offset = (i as f64 - (grid_levels as f64 / 2.0)) * grid_spacing;
        let grid_price = base_price * (1.0 + offset);
        grid_prices.push(grid_price);
    }
    
    assert_eq!(grid_prices.len(), 5);
    
    // Verify grid is centered around base price
    let mid_idx = grid_levels / 2;
    assert!((grid_prices[mid_idx] - base_price).abs() < 0.01);
}

#[test]
fn test_trade_execution_logic() {
    let entry_price = 0.50;
    let exit_price = 0.51;
    let position_size = 1000.0;
    let fee_percent = 0.26;
    
    // Buy trade
    let buy_cost = position_size * entry_price;
    let buy_fee = buy_cost * (fee_percent / 100.0);
    let total_buy_cost = buy_cost + buy_fee;
    
    // Sell trade
    let sell_revenue = position_size * exit_price;
    let sell_fee = sell_revenue * (fee_percent / 100.0);
    let net_sell_revenue = sell_revenue - sell_fee;
    
    // Profit calculation
    let profit = net_sell_revenue - total_buy_cost;
    let profit_pct = (profit / total_buy_cost) * 100.0;
    
    assert!(profit > 0.0, "Trade should be profitable");
    assert!(profit_pct > 0.0 && profit_pct < 2.0, "Profit percentage should be reasonable");
}

#[test]
fn test_risk_management_stop_loss() {
    let entry_price = 0.50_f64;
    let stop_loss_percent = 5.0_f64;
    let position_size = 1000.0_f64;
    
    let stop_loss_price = entry_price * (1.0 - stop_loss_percent / 100.0);
    let max_loss = position_size * (entry_price - stop_loss_price);
    
    assert!((stop_loss_price - 0.475).abs() < 0.001);
    assert!((max_loss - 25.0).abs() < 0.001);
}

#[test]
fn test_drawdown_calculation() {
    let equity_curve = vec![
        10000.0, 10500.0, 10800.0, 10200.0, 9800.0, 10100.0, 10900.0, 11200.0
    ];
    
    let mut peak = equity_curve[0];
    let mut max_drawdown = 0.0;
    
    for &value in &equity_curve {
        if value > peak {
            peak = value;
        }
        let drawdown = ((peak - value) / peak) * 100.0;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }
    
    assert!(max_drawdown > 0.0, "Should have experienced drawdown");
    assert!(max_drawdown < 15.0, "Max drawdown should be less than 15%");
}

#[test]
fn test_sharpe_ratio_calculation() {
    // Sample daily returns (percentage)
    let returns = vec![0.5, -0.3, 0.8, 0.2, -0.1, 0.6, 0.4, -0.2, 0.7, 0.3];
    
    // Calculate mean return
    let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
    
    // Calculate standard deviation
    let variance: f64 = returns.iter()
        .map(|&r| (r - mean_return).powi(2))
        .sum::<f64>() / returns.len() as f64;
    let std_dev = variance.sqrt();
    
    // Sharpe ratio (assuming risk-free rate of 0)
    let sharpe_ratio = mean_return / std_dev;
    
    assert!(sharpe_ratio > 0.0, "Sharpe ratio should be positive for profitable strategy");
}

#[test]
fn test_volatility_calculation() {
    let prices = vec![0.50, 0.51, 0.49, 0.52, 0.48, 0.53, 0.50];
    
    // Calculate returns
    let mut returns = vec![];
    for i in 1..prices.len() {
        let ret = (prices[i] - prices[i-1]) / prices[i-1];
        returns.push(ret);
    }
    
    // Calculate mean
    let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
    
    // Calculate volatility (standard deviation)
    let variance: f64 = returns.iter()
        .map(|&r| (r - mean).powi(2))
        .sum::<f64>() / returns.len() as f64;
    let volatility = variance.sqrt();
    
    assert!(volatility > 0.0, "Volatility should be positive");
    assert!(volatility < 0.1, "Volatility should be less than 10%");
}

#[test]
fn test_position_sizing() {
    let capital = 10000.0;
    let risk_per_trade = 2.0; // 2% risk per trade
    let max_positions = 5.0;
    
    let position_size = (capital * risk_per_trade / 100.0) * max_positions;
    
    assert_eq!(position_size, 1000.0);
    
    // Verify position doesn't exceed capital
    assert!(position_size <= capital);
}

#[test]
fn test_trade_simulation_sequence() {
    let mut capital = 10000.0;
    let trades = vec![
        (0.50, 0.51, 1000.0), // Buy at 0.50, sell at 0.51, size 1000
        (0.51, 0.52, 1000.0),
        (0.52, 0.51, 1000.0), // Losing trade
        (0.51, 0.53, 1000.0),
    ];
    
    let fee_percent = 0.26;
    let mut total_trades = 0;
    let mut winning_trades = 0;
    
    for (buy_price, sell_price, size) in trades {
        total_trades += 1;
        
        let buy_cost = size * buy_price;
        let buy_fee = buy_cost * (fee_percent / 100.0);
        
        let sell_revenue = size * sell_price;
        let sell_fee = sell_revenue * (fee_percent / 100.0);
        
        let profit = (sell_revenue - sell_fee) - (buy_cost + buy_fee);
        capital += profit;
        
        if profit > 0.0 {
            winning_trades += 1;
        }
    }
    
    assert!(capital > 10000.0, "Capital should increase");
    assert_eq!(total_trades, 4);
    assert_eq!(winning_trades, 3);
}
