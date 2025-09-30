// Simple working backtest runner to demonstrate the system

use clap::{Parser, Subcommand};
use grid_trading_bot::{BacktestBuilder};
use chrono::{Utc, Duration};
use serde::{Serialize, Deserialize};
use std::fs;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "backtest")]
#[command(about = "Grid Trading Backtest System")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Quick demo backtest
    Demo {
        /// Trading pair (e.g., XRPGBP)
        #[arg(short, long, default_value = "XRPGBP")]
        pair: String,
    },
    /// List available pairs
    List,
}

#[derive(Serialize, Deserialize)]
pub struct SimpleStrategy {
    pub trading_pair: String,
    pub grid_levels: usize,
    pub grid_spacing: f64,
    pub expected_return: f64,
    pub total_trades: usize,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Demo { pair } => {
            info!("🚀 Running demo backtest for {}", pair);
            run_demo_backtest(&pair).await?;
        }
        Commands::List => {
            info!("📋 Available pairs: XRPGBP, BTCGBP, ETHGBP");
        }
    }
    
    Ok(())
}

async fn run_demo_backtest(pair: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure strategies directory exists
    fs::create_dir_all("strategies")?;
    
    info!("📊 Setting up backtest for {}", pair);
    
    // Create engine with simple configuration
    let mut engine = BacktestBuilder::new()
        .with_initial_capital(10000.0)
        .with_grid_levels(5)
        .with_grid_spacing(0.01) // 1%
        .with_markov_analysis(true)
        .build();

    // Use last 30 days of data
    let end_date = Utc::now();
    let start_date = end_date - Duration::days(30);
    
    info!("⏰ Backtesting {} from {} to {}", 
          pair, 
          start_date.format("%Y-%m-%d"), 
          end_date.format("%Y-%m-%d"));

    // Run the backtest
    let result = engine.run_backtest(
        pair,
        start_date,
        end_date,
        60, // 1-hour timeframe
    ).await?;

    info!("✅ Backtest completed!");
    info!("📈 Results:");
    info!("   Total Return: {:.2}%", result.performance_metrics.total_return_pct);
    info!("   Total Trades: {}", result.performance_metrics.total_trades);
    info!("   Win Rate: {:.1}%", result.performance_metrics.win_rate_pct);
    info!("   Sharpe Ratio: {:.2}", result.performance_metrics.sharpe_ratio);
    info!("   Max Drawdown: {:.2}%", result.performance_metrics.max_drawdown_pct);
    info!("   Total Fees: £{:.2}", result.performance_metrics.total_fees_paid);

    // Create simple strategy file
    let strategy = SimpleStrategy {
        trading_pair: pair.to_string(),
        grid_levels: 5,
        grid_spacing: 0.01,
        expected_return: result.performance_metrics.total_return_pct,
        total_trades: result.performance_metrics.total_trades,
        generated_at: Utc::now(),
    };

    // Save strategy
    let filename = format!("strategies/{}_strategy.json", pair.to_lowercase());
    let json = serde_json::to_string_pretty(&strategy)?;
    fs::write(&filename, json)?;
    
    info!("💾 Strategy saved: {}", filename);
    
    if result.performance_metrics.total_trades > 0 {
        info!("🎉 Strategy is ready for live trading!");
        info!("💡 Next: Run 'cargo run --bin trade demo --pair {}' to simulate live trading", pair);
    } else {
        warn!("⚠️  No trades generated - try a different time period or pair");
    }

    Ok(())
}