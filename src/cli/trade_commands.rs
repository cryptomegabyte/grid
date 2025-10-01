// Trade command implementations - Phase 2 with config
use tracing::{info, warn, error};
use std::path::Path;
use std::fs;
use serde::{Deserialize, Serialize};
use grid_trading_bot::CliConfig;

#[derive(Serialize, Deserialize, Clone)]
pub struct SimpleStrategy {
    pub trading_pair: String,
    pub grid_levels: usize,
    pub grid_spacing: f64,
    #[serde(default)]
    pub expected_return: f64,
    #[serde(default)]
    pub total_trades: usize,
}

pub async fn start_trading(
    capital: f64,
    hours: Option<f64>,
    minutes: Option<f64>,
    pairs: Option<String>,
    dry_run: bool,
    config: &CliConfig,
) -> grid_trading_bot::TradingResult<()> {
    use grid_trading_bot::core::LiveTradingEngine;
    use grid_trading_bot::PreFlightValidator;
    use std::time::Duration;

    if dry_run {
        info!("🧪 DRY RUN mode (paper trading)");
    } else {
        info!("🚀 LIVE TRADING");
        warn!("⚠️  Real money!");
    }

    let final_capital = if capital != 500.0 { capital } else { config.trading.default_capital };
    
    // Run pre-flight validation
    info!("");
    let validator = PreFlightValidator::new(config.clone());
    let validation = if dry_run {
        validator.validate_for_backtesting().await
    } else {
        validator.validate_for_trading(final_capital).await
    };
    
    validation.display();
    
    if !validation.passed {
        error!("");
        error!("❌ Pre-flight validation failed. Cannot proceed.");
        return Err(grid_trading_bot::TradingError::ValidationFailed(
            "Critical validation checks did not pass".to_string()
        ));
    }
    
    info!("");
    info!("💰 Capital: £{:.2}", final_capital);
    info!("⚙️  Max position: {:.1}%", config.trading.max_position_size * 100.0);

    let duration = if let Some(h) = hours {
        Some(Duration::from_secs_f64(h * 3600.0))
    } else if let Some(m) = minutes {
        Some(Duration::from_secs_f64(m * 60.0))
    } else {
        None
    };

    let strategies = if let Some(p) = pairs {
        load_specific_strategies(&p)?
    } else {
        load_all_strategies()?
    };

    if strategies.is_empty() {
        error!("❌ No strategies!");
        return Err("No strategies".into());
    }

    info!("📊 Loaded {} strategies", strategies.len());
    
    // Initialize the trading engine
    let mut engine = LiveTradingEngine::new(final_capital)
        .with_simulation_engine(true)
        .with_real_data(!dry_run);
    
    info!("✅ Engine initialized");
    
    // Load optimized strategies from the strategies directory
    info!("📂 Loading optimized strategies from 'strategies' directory...");
    match engine.load_optimized_strategies(std::path::Path::new("strategies")) {
        Ok(count) => {
            info!("✅ Loaded {} optimized strategies", count);
            if count == 0 {
                error!("❌ No optimized strategies found!");
                return Err("No optimized strategies found in strategies directory".into());
            }
        }
        Err(e) => {
            error!("❌ Failed to load strategies: {}", e);
            return Err(format!("Failed to load strategies: {}", e).into());
        }
    }
    
    // Start the trading loop
    info!("");
    info!("🚀 Starting trading engine...");
    if let Some(duration) = duration {
        info!("⏱️  Duration: {:.1} hours", duration.as_secs_f64() / 3600.0);
        engine.start_simulation_with_duration(duration).await
            .map_err(|e| grid_trading_bot::TradingError::from(format!("Trading engine failed: {}", e)))?;
    } else {
        info!("⏱️  Duration: Indefinite (press Ctrl+C to stop)");
        engine.start_simulation().await
            .map_err(|e| grid_trading_bot::TradingError::from(format!("Trading engine failed: {}", e)))?;
    }
    
    // Display final summary
    info!("");
    info!("🏁 Trading session completed");
    let summary = engine.get_portfolio_summary();
    info!("📊 Final Summary:");
    info!("   Total Value: £{:.2}", summary.total_value);
    info!("   Return: {:+.2}%", summary.total_return);
    info!("   Total Trades: {}", summary.total_trades);
    info!("   Total Fees: £{:.2}", summary.total_fees);
    
    Ok(())
}

pub async fn stop_trading(_force: bool) -> grid_trading_bot::TradingResult<()> {
    info!("🛑 Stopping...");
    Ok(())
}

pub async fn pause_trading() -> grid_trading_bot::TradingResult<()> {
    info!("⏸️  Pausing...");
    Ok(())
}

pub async fn resume_trading() -> grid_trading_bot::TradingResult<()> {
    info!("▶️  Resuming...");
    Ok(())
}

fn load_all_strategies() -> Result<Vec<SimpleStrategy>, grid_trading_bot::TradingError> {
    let dir = "strategies";
    if !Path::new(dir).exists() {
        return Err("Strategies dir not found".into());
    }
    let mut strats = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if path.to_string_lossy().contains("_optimized.json") {
                if let Ok(s) = load_strategy(&path) {
                    strats.push(s);
                }
            }
        }
    }
    Ok(strats)
}

fn load_specific_strategies(pairs: &str) -> Result<Vec<SimpleStrategy>, grid_trading_bot::TradingError> {
    let mut strats = Vec::new();
    for pair in pairs.split(',').map(|s| s.trim()) {
        let file = format!("strategies/{}_optimized.json", pair.to_lowercase());
        if Path::new(&file).exists() {
            if let Ok(s) = load_strategy(Path::new(&file)) {
                strats.push(s);
            }
        }
    }
    Ok(strats)
}

fn load_strategy(path: &Path) -> Result<SimpleStrategy, grid_trading_bot::TradingError> {
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}
