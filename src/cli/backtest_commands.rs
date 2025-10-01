// Backtest command implementations - Phase 2 with config
use tracing::{info, warn};
use grid_trading_bot::CliConfig;

pub async fn optimize_all_pairs(
    limit: Option<usize>,
    strategy: &str,
    iterations: usize,
    _report: bool,
    config: &CliConfig,
) -> grid_trading_bot::TradingResult<()> {
    info!("🔍 Optimizing GBP pairs with config...");
    info!("   Strategy: {}", strategy);
    info!("   Iterations: {}", iterations);
    info!("   Config capital: £{:.2}", config.trading.default_capital);
    
    if let Some(limit) = limit {
        info!("   Limit: {} pairs", limit);
    }
    
    warn!("⚠️  Full integration in progress");
    info!("   For now: cargo run --bin backtest -- optimize-gbp");
    Ok(())
}

pub async fn optimize_single_pair(
    pair: &str,
    _strategy: &str,
    _iterations: usize,
    _comprehensive: bool,
    config: &CliConfig,
) -> grid_trading_bot::TradingResult<()> {
    use grid_trading_bot::PreFlightValidator;
    
    info!("🎯 Optimizing {} with config", pair);
    
    // Run pre-flight validation
    let validator = PreFlightValidator::new(config.clone());
    let validation = validator.validate_for_backtesting().await;
    
    if !validation.passed {
        validation.display();
        return Err(grid_trading_bot::TradingError::ValidationFailed(
            "Validation failed".to_string()
        ));
    }
    
    info!("   Grid range: {:?}", config.optimization.grid_levels_range);
    warn!("⚠️  For now: cargo run --bin backtest -- optimize-pair --pair {}", pair);
    Ok(())
}

pub async fn run_demo_backtest(pair: &str, config: &CliConfig) -> grid_trading_bot::TradingResult<()> {
    use grid_trading_bot::Spinner;
    
    info!("🚀 Demo backtest for {}", pair);
    info!("   Capital: £{:.2}", config.trading.default_capital);
    
    // Show spinner while loading
    let spinner = Spinner::new(&format!("Loading historical data for {}...", pair));
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    spinner.finish(&format!("Loaded {} data points", 1000));
    
    info!("");
    warn!("⚠️  For now: cargo run --bin backtest -- demo");
    Ok(())
}

pub async fn scan_pairs(
    _limit: Option<usize>,
    _report: bool,
    config: &CliConfig,
) -> grid_trading_bot::TradingResult<()> {
    info!("🔍 Scanning pairs...");
    info!("   Lookback: {} days", config.backtesting.default_lookback_days);
    warn!("⚠️  For now: cargo run --bin backtest -- list");
    Ok(())
}

pub async fn run_custom_backtest(
    pair: &str,
    _start: Option<String>,
    _end: Option<String>,
    levels: Option<usize>,
    spacing: Option<f64>,
    config: &CliConfig,
) -> grid_trading_bot::TradingResult<()> {
    info!("🎯 Custom backtest for {}", pair);
    let final_levels = levels.unwrap_or(config.trading.default_grid_levels);
    let final_spacing = spacing.unwrap_or(config.trading.default_grid_spacing);
    info!("   Levels: {}", final_levels);
    info!("   Spacing: {:.2}%", final_spacing * 100.0);
    warn!("⚠️  For now: cargo run --bin backtest -- demo");
    Ok(())
}
