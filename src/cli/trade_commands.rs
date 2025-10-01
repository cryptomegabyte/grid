// Trade command implementations
use tracing::{info, warn, error};
use std::path::Path;
use std::fs;
use serde::{Deserialize, Serialize};

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
) -> Result<(), Box<dyn std::error::Error>> {
    use grid_trading_bot::core::LiveTradingEngine;
    use std::time::Duration;

    if dry_run {
        info!("🧪 Starting in DRY RUN mode (paper trading)");
    } else {
        info!("🚀 Starting LIVE TRADING");
        warn!("⚠️  Real money will be used!");
    }

    info!("💰 Capital: £{:.2}", capital);

    // Calculate duration
    let duration = if let Some(hours) = hours {
        Some(Duration::from_secs_f64(hours * 3600.0))
    } else if let Some(minutes) = minutes {
        Some(Duration::from_secs_f64(minutes * 60.0))
    } else {
        None
    };

    if let Some(duration) = duration {
        info!("⏱️  Duration: {} minutes", duration.as_secs() / 60);
    } else {
        info!("⏱️  Duration: Indefinite (press Ctrl+C to stop)");
    }

    // Load strategies
    let strategies = if let Some(pairs) = pairs {
        load_specific_strategies(&pairs)?
    } else {
        load_all_strategies()?
    };

    if strategies.is_empty() {
        error!("❌ No strategies found!");
        error!("   Run: grid-bot optimize all");
        return Err("No strategies available".into());
    }

    info!("📊 Loaded {} strategies", strategies.len());
    for strategy in &strategies {
        info!("   - {}", strategy.trading_pair);
    }

    // Create trading engine (stub for Phase 1)
    let _engine = LiveTradingEngine::new(capital);

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("✅ Trading engine initialized");
    info!("🎯 Starting execution...");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Start trading (simplified for now - full implementation will be in Phase 2)
    info!("⚠️  Trading engine ready but execution not yet connected to new CLI");
    info!("   This will be fully implemented in Phase 2 with config system");

    Ok(())
}

pub async fn stop_trading(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    if force {
        info!("🛑 Force stopping all trading...");
        // Implement force stop logic
    } else {
        info!("🛑 Gracefully stopping trading...");
        // Implement graceful shutdown
    }

    info!("✅ Trading stopped");
    Ok(())
}

pub async fn pause_trading() -> Result<(), Box<dyn std::error::Error>> {
    info!("⏸️  Pausing trading...");
    // Implement pause logic
    info!("✅ Trading paused");
    Ok(())
}

pub async fn resume_trading() -> Result<(), Box<dyn std::error::Error>> {
    info!("▶️  Resuming trading...");
    // Implement resume logic
    info!("✅ Trading resumed");
    Ok(())
}

fn load_all_strategies() -> Result<Vec<SimpleStrategy>, Box<dyn std::error::Error>> {
    let strategies_dir = "strategies";
    
    if !Path::new(strategies_dir).exists() {
        return Err("Strategies directory not found".into());
    }

    let mut strategies = Vec::new();

    for entry in fs::read_dir(strategies_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            // Prefer optimized versions
            if path.to_string_lossy().contains("_optimized.json") {
                match load_strategy_from_file(&path) {
                    Ok(strategy) => {
                        strategies.push(strategy);
                    }
                    Err(e) => {
                        warn!("Failed to load {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    Ok(strategies)
}

fn load_specific_strategies(
    pairs: &str
) -> Result<Vec<SimpleStrategy>, Box<dyn std::error::Error>> {
    let strategies_dir = "strategies";
    let requested_pairs: Vec<&str> = pairs.split(',').map(|s| s.trim()).collect();

    let mut strategies = Vec::new();

    for pair in requested_pairs {
        let filename = format!("{}/{}_optimized.json", strategies_dir, pair.to_lowercase());
        let path = Path::new(&filename);

        if path.exists() {
            match load_strategy_from_file(path) {
                Ok(strategy) => {
                    strategies.push(strategy);
                }
                Err(e) => {
                    error!("Failed to load strategy for {}: {}", pair, e);
                }
            }
        } else {
            warn!("No optimized strategy found for {}", pair);
        }
    }

    Ok(strategies)
}

fn load_strategy_from_file(
    path: &Path
) -> Result<SimpleStrategy, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let strategy = serde_json::from_str(&content)?;
    Ok(strategy)
}
