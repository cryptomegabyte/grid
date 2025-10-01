// Live Trading Engine with Optimized Strategy Loading

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};
use grid_trading_bot::core::{LiveTradingEngine, GridMode};
use chrono::Utc;

#[derive(Parser)]
#[command(name = "trade")]
#[command(about = "Grid Trading Live Engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start live trading simulation with all optimized strategies
    Start {
        /// Initial capital in GBP
        #[arg(short, long, default_value = "500")]
        capital: f64,
        /// Strategies directory
        #[arg(short, long, default_value = "optimized_strategies")]
        strategies_dir: String,
        /// Trading duration in hours (optional, runs indefinitely if not specified)
        #[arg(long)]
        hours: Option<f64>,
        /// Trading duration in minutes (optional, alternative to hours)
        #[arg(short, long)]
        minutes: Option<f64>,
        /// Use simulation engine with local order book (paper trading mode)
        #[arg(long)]
        simulate: bool,
    },
    /// Demo single pair trading
    Demo {
        /// Trading pair
        #[arg(short, long, default_value = "XRPGBP")]
        pair: String,
    },
    /// List available optimized strategies
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
        Commands::Start { capital, strategies_dir, hours, minutes, simulate } => {
            let duration = calculate_trading_duration(hours, minutes);
            if let Some(duration) = duration {
                info!("🚀 Starting live trading simulation with £{:.2} for {:.1} hours", capital, duration.as_secs_f64() / 3600.0);
            } else {
                info!("🚀 Starting live trading simulation with £{:.2} (indefinite)", capital);
            }
            if simulate {
                info!("🎮 Simulation engine enabled - Using local order book");
            }
            run_live_trading_simulation(capital, &strategies_dir, duration, simulate).await?;
        }
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

fn calculate_trading_duration(hours: Option<f64>, minutes: Option<f64>) -> Option<Duration> {
    match (hours, minutes) {
        (Some(h), None) => Some(Duration::from_secs_f64(h * 3600.0)),
        (None, Some(m)) => Some(Duration::from_secs_f64(m * 60.0)),
        (Some(h), Some(_m)) => {
            warn!("⚠️  Both hours and minutes specified, using hours ({:.1}h)", h);
            Some(Duration::from_secs_f64(h * 3600.0))
        }
        (None, None) => None, // Run indefinitely
    }
}

async fn run_live_trading_simulation(capital: f64, strategies_dir: &str, duration: Option<Duration>, use_simulation: bool) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔧 Initializing live trading engine with £{:.2} capital", capital);
    
    // Create trading engine with real market data
    let mut engine = LiveTradingEngine::new(capital)
        .with_real_data(true)
        .with_grid_mode(GridMode::VolatilityAdaptive);
    
    // Enable simulation engine if requested
    if use_simulation {
        engine = engine.with_simulation_engine(true);
        info!("🎮 Simulation engine initialized with local order book");
    }
    
    // Load all optimized strategies
    let loaded_count = engine.load_optimized_strategies(strategies_dir)?;
    
    if loaded_count == 0 {
        warn!("⚠️  No optimized strategies found in {}", strategies_dir);
        warn!("💡 Run 'make backtest' first to generate optimized strategies");
        return Ok(());
    }
    
    info!("✅ Loaded {} optimized strategies", loaded_count);
    
    if let Some(duration) = duration {
        info!("🎯 Starting LIVE trading with real market data for {:.1} hours - Press Ctrl+C to stop early", duration.as_secs_f64() / 3600.0);
        info!("⏰ Trading will automatically stop at {}", (Utc::now() + chrono::Duration::from_std(duration).unwrap()).format("%H:%M:%S UTC"));
    } else {
        info!("🎯 Starting LIVE trading with real market data - Press Ctrl+C to stop");
    }
    
    info!("📡 Using adaptive grid spacing based on real market volatility");
    info!("🔗 Connecting to Kraken WebSocket for real-time price feeds...");
    
    // Set up graceful shutdown with optional duration
    let ctrl_c = tokio::signal::ctrl_c();
    
    let simulation_result = if let Some(duration) = duration {
        // Run with time limit
        tokio::select! {
            result = engine.start_simulation_with_duration(duration) => result,
            _ = ctrl_c => {
                info!("🛑 Received shutdown signal (early stop)");
                Ok(())
            }
        }
    } else {
        // Run indefinitely
        tokio::select! {
            result = engine.start_simulation() => result,
            _ = ctrl_c => {
                info!("🛑 Received shutdown signal");
                Ok(())
            }
        }
    };
    
    // Show final summary
    match simulation_result {
        Ok(_) => {
            if duration.is_some() {
                info!("⏰ Trading session completed after scheduled duration");
            } else {
                info!("✅ Simulation completed successfully");
            }
        }
        Err(e) => error!("❌ Simulation error: {}", e),
    }
    
    let summary = engine.get_portfolio_summary();
    info!("📊 Final Portfolio Summary:");
    info!("   💰 Total Value: £{:.2}", summary.total_value);
    info!("   📈 Total Return: {:.2}%", summary.total_return);
    info!("   🔄 Total Trades: {}", summary.total_trades);
    info!("   💸 Total Fees: £{:.2}", summary.total_fees);
    
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
    // Check optimized strategies first
    if std::path::Path::new("optimized_strategies").exists() {
        info!("📋 Optimized Strategies (Ready for Live Trading):");
        
        let mut found_any = false;
        for entry in fs::read_dir("optimized_strategies")? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(strategy) = serde_json::from_str::<grid_trading_bot::core::live_trading::OptimizedStrategy>(&content) {
                        found_any = true;
                        info!("🎯 {}", strategy.trading_pair);
                        info!("   Expected Return: {:.2}%", strategy.expected_return * 100.0);
                        info!("   Sharpe Ratio: {:.2}", strategy.sharpe_ratio);
                        info!("   Max Drawdown: {:.2}%", strategy.max_drawdown * 100.0);
                        info!("   Grid Levels: {}", strategy.grid_levels);
                        info!("   Generated: {}", strategy.generated_at.format("%Y-%m-%d %H:%M"));
                        info!("");
                    }
                }
            }
        }
        
        if !found_any {
            warn!("⚠️  No optimized strategies found");
        } else {
            info!("✅ Found {} optimized strategies ready for live trading", 
                fs::read_dir("optimized_strategies")?.count());
            return Ok(());
        }
    }
    
    // Fallback to basic strategies
    if std::path::Path::new("strategies").exists() {
        info!("📋 Basic Strategies (Demo Only):");
        
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
        
        if found_any {
            warn!("💡 These are basic demo strategies. Run 'make backtest' to generate optimized strategies for live trading.");
        }
    }
    
    if !std::path::Path::new("optimized_strategies").exists() && !std::path::Path::new("strategies").exists() {
        error!("❌ No strategies found");
        info!("💡 Run 'make backtest' to generate optimized strategies");
    }
    
    Ok(())
}