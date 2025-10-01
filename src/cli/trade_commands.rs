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
) -> Result<(), Box<dyn std::error::Error>> {
    use grid_trading_bot::core::LiveTradingEngine;
    use std::time::Duration;

    if dry_run {
        info!("🧪 DRY RUN mode (paper trading)");
    } else {
        info!("🚀 LIVE TRADING");
        warn!("⚠️  Real money!");
        
        if !config.has_valid_api_keys() {
            error!("❌ API keys not configured!");
            return Err("Invalid API configuration".into());
        }
    }

    let final_capital = if capital != 500.0 { capital } else { config.trading.default_capital };
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
    let _engine = LiveTradingEngine::new(final_capital);
    
    info!("✅ Engine initialized");
    warn!("⚠️  Full execution in Phase 3");
    Ok(())
}

pub async fn stop_trading(_force: bool) -> Result<(), Box<dyn std::error::Error>> {
    info!("🛑 Stopping...");
    Ok(())
}

pub async fn pause_trading() -> Result<(), Box<dyn std::error::Error>> {
    info!("⏸️  Pausing...");
    Ok(())
}

pub async fn resume_trading() -> Result<(), Box<dyn std::error::Error>> {
    info!("▶️  Resuming...");
    Ok(())
}

fn load_all_strategies() -> Result<Vec<SimpleStrategy>, Box<dyn std::error::Error>> {
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

fn load_specific_strategies(pairs: &str) -> Result<Vec<SimpleStrategy>, Box<dyn std::error::Error>> {
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

fn load_strategy(path: &Path) -> Result<SimpleStrategy, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}
