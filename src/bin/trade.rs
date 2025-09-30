// Simple live trading demo

use clap::{Parser, Subcommand};
// Removed unused import
use serde::{Deserialize, Serialize};
use std::fs;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};

#[derive(Parser)]
#[command(name = "trade")]
#[command(about = "Grid Trading Live Engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Demo live trading simulation
    Demo {
        /// Trading pair
        #[arg(short, long, default_value = "XRPGBP")]
        pair: String,
    },
    /// List available strategies
    List,
}

#[derive(Serialize, Deserialize)]
struct SimpleStrategy {
    trading_pair: String,
    grid_levels: usize,
    grid_spacing: f64,
    expected_return: f64,
    total_trades: usize,
    generated_at: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Demo { pair } => {
            info!("🎯 Starting demo live trading for {}", pair);
            run_demo_trading(&pair).await?;
        }
        Commands::List => {
            list_strategies().await?;
        }
    }
    
    Ok(())
}

async fn run_demo_trading(pair: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Load strategy
    let filename = format!("strategies/{}_strategy.json", pair.to_lowercase());
    
    if !std::path::Path::new(&filename).exists() {
        error!("❌ No strategy found for {}!", pair);
        error!("💡 Run 'cargo run --bin simple_backtest demo --pair {}' first", pair);
        return Ok(());
    }
    
    let content = fs::read_to_string(&filename)?;
    let strategy: SimpleStrategy = serde_json::from_str(&content)?;
    
    info!("📊 Strategy loaded for {}", pair);
    info!("   Grid Levels: {}", strategy.grid_levels);
    info!("   Grid Spacing: {:.1}%", strategy.grid_spacing * 100.0);
    info!("   Expected Return: {:.2}%", strategy.expected_return);
    info!("   Backtest Trades: {}", strategy.total_trades);
    
    info!("🧪 DEMO MODE - Simulating live trading for 30 seconds...");
    
    // Simulate price movements and trading
    let mut price = 2.0; // Starting price
    let base_price = price;
    
    info!("🎯 Grid levels set around base price £{:.4}", base_price);
    
    // Calculate grid levels
    let grid_size = base_price * strategy.grid_spacing;
    let mut buy_levels = Vec::new();
    let mut sell_levels = Vec::new();
    
    for i in 1..=strategy.grid_levels {
        buy_levels.push(base_price - (i as f64 * grid_size));
        sell_levels.push(base_price + (i as f64 * grid_size));
    }
    
    info!("📏 Buy levels: {:?}", buy_levels.iter().map(|p| format!("£{:.4}", p)).collect::<Vec<_>>());
    info!("📏 Sell levels: {:?}", sell_levels.iter().map(|p| format!("£{:.4}", p)).collect::<Vec<_>>());
    
    let mut trades_executed = 0;
    let start_time = std::time::Instant::now();
    
    while start_time.elapsed().as_secs() < 30 {
        // Simulate price movement
        let change = (rand::random::<f64>() - 0.5) * 0.01; // ±0.5% random change
        price += price * change;
        
        info!("💹 Current price: £{:.6}", price);
        
        // Check for buy signals
        for &buy_level in &buy_levels {
            if price <= buy_level {
                info!("🟢 BUY SIGNAL at £{:.6} (grid level £{:.6})", price, buy_level);
                trades_executed += 1;
                break;
            }
        }
        
        // Check for sell signals  
        for &sell_level in &sell_levels {
            if price >= sell_level {
                info!("🔴 SELL SIGNAL at £{:.6} (grid level £{:.6})", price, sell_level);
                trades_executed += 1;
                break;
            }
        }
        
        sleep(Duration::from_secs(2)).await;
    }
    
    info!("🏁 Demo completed!");
    info!("📊 Demo Results:");
    info!("   Simulated Trades: {}", trades_executed);
    info!("   Final Price: £{:.6}", price);
    info!("   Price Change: {:.2}%", ((price - base_price) / base_price) * 100.0);
    
    if trades_executed > 0 {
        info!("✅ Grid trading system is working!");
        info!("💡 This demonstrates how the system would detect and execute trades");
    } else {
        warn!("⚠️  No trades triggered in demo - price didn't move enough");
    }
    
    Ok(())
}

async fn list_strategies() -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new("strategies").exists() {
        info!("❌ No strategies directory found");
        info!("💡 Run 'cargo run --bin simple_backtest demo' to generate strategies");
        return Ok(());
    }
    
    info!("📋 Available Strategies:");
    
    let mut found_any = false;
    for entry in fs::read_dir("strategies")? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(strategy) = serde_json::from_str::<SimpleStrategy>(&content) {
                    found_any = true;
                    info!("🎯 {}", strategy.trading_pair);
                    info!("   Expected Return: {:.2}%", strategy.expected_return);
                    info!("   Grid Spacing: {:.1}%", strategy.grid_spacing * 100.0);
                    info!("   Generated: {}", strategy.generated_at.format("%Y-%m-%d %H:%M"));
                    info!("");
                }
            }
        }
    }
    
    if !found_any {
        info!("❌ No valid strategies found");
        info!("💡 Run 'cargo run --bin simple_backtest demo' to generate strategies");
    }
    
    Ok(())
}