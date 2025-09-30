// Simple working backtest runner to demonstrate the system

use clap::{Parser, Subcommand};
use grid_trading_bot::BacktestBuilder;
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
    /// Scan all GBP pairs for backtesting
    ScanGbp {
        /// Maximum number of pairs to test (default: all)
        #[arg(short, long)]
        limit: Option<usize>,
        /// Generate portfolio comparison report
        #[arg(short, long)]
        report: bool,
    },
}

#[derive(Serialize, Deserialize)]
pub struct SimpleStrategy {
    pub trading_pair: String,
    pub grid_levels: usize,
    pub grid_spacing: f64,
    pub expected_return: f64,
    pub total_trades: usize,
    pub win_rate: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub total_fees: f64,
    pub markov_confidence: f64,
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
            info!("📋 Fetching available GBP pairs from Kraken...");
            list_available_pairs().await?;
        }
        Commands::ScanGbp { limit, report } => {
            info!("🔍 Scanning all GBP pairs for backtesting...");
            scan_gbp_pairs(limit, report).await?;
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
        win_rate: result.performance_metrics.win_rate_pct,
        sharpe_ratio: result.performance_metrics.sharpe_ratio,
        max_drawdown: result.performance_metrics.max_drawdown_pct,
        total_fees: result.performance_metrics.total_fees_paid,
        markov_confidence: 0.0,
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

async fn list_available_pairs() -> Result<(), Box<dyn std::error::Error>> {
    use grid_trading_bot::clients::get_gbp_pairs;
    
    let pairs = get_gbp_pairs().await?;
    
    info!("📊 Found {} available GBP trading pairs:", pairs.len());
    
    for pair in pairs {
        let base_name = pair.base.replace("X", "").replace("Z", "");
        info!("   • {} ({})", pair.alt_name, base_name);
    }
    
    Ok(())
}

async fn scan_gbp_pairs(limit: Option<usize>, generate_report: bool) -> Result<(), Box<dyn std::error::Error>> {
    use grid_trading_bot::clients::get_gbp_pair_names;
    use std::time::Instant;
    
    let start_time = Instant::now();
    
    // Get all GBP pairs
    let all_pairs = get_gbp_pair_names().await?;
    let pairs_to_test = if let Some(limit) = limit {
        all_pairs.into_iter().take(limit).collect::<Vec<_>>()
    } else {
        all_pairs
    };
    
    info!("🎯 Testing {} GBP pairs:", pairs_to_test.len());
    
    // Ensure strategies directory exists
    fs::create_dir_all("strategies")?;
    
    let mut results: Vec<(String, BacktestSummary)> = Vec::new();
    let mut successful_backtests = 0;
    
    for (index, pair) in pairs_to_test.iter().enumerate() {
        info!("📈 [{}/{}] Backtesting {}...", index + 1, pairs_to_test.len(), pair);
        
        match run_single_backtest(pair).await {
            Ok(result) => {
                if result.total_trades > 0 {
                    successful_backtests += 1;
                    info!("   ✅ {} trades, {:.2}% return", result.total_trades, result.total_return * 100.0);
                    results.push((pair.to_string(), result));
                } else {
                    info!("   ⚠️  No trades generated");
                }
            }
            Err(e) => {
                warn!("   ❌ Failed: {}", e);
            }
        }
    }
    
    let elapsed = start_time.elapsed();
    
    info!("🏁 Scan completed in {:.1}s", elapsed.as_secs_f64());
    info!("📊 Results: {}/{} pairs generated strategies", successful_backtests, pairs_to_test.len());
    
    if generate_report && !results.is_empty() {
        generate_portfolio_report(&results).await?;
    }
    
    // Show top performers
    if !results.is_empty() {
        results.sort_by(|a, b| b.1.total_return.partial_cmp(&a.1.total_return).unwrap());
        
        info!("🏆 Top 5 performers:");
        for (pair, result) in results.iter().take(5) {
            info!("   • {}: {:.2}% return, {} trades", pair, result.total_return * 100.0, result.total_trades);
        }
    }
    
    Ok(())
}

#[derive(Debug, Clone)]
struct BacktestSummary {
    total_return: f64,
    total_trades: usize,
    win_rate: f64,
    sharpe_ratio: f64,
    max_drawdown: f64,
}

async fn run_single_backtest(pair: &str) -> Result<BacktestSummary, Box<dyn std::error::Error>> {
    // Set up backtest parameters
    let end_date = Utc::now();
    let start_date = end_date - Duration::days(30);
    
    let builder = BacktestBuilder::new()
        .with_grid_levels(5)
        .with_grid_spacing(0.01);
    
    let mut engine = builder.build();
    let result = engine.run_backtest(pair, start_date, end_date, 60).await?;
    
    // Save strategy if successful
    if result.performance_metrics.total_trades > 0 {
        let strategy = SimpleStrategy {
            trading_pair: pair.to_string(),
            grid_levels: 5,
            grid_spacing: 0.01,
            expected_return: result.performance_metrics.total_return_pct,
            total_trades: result.performance_metrics.total_trades,
            win_rate: result.performance_metrics.win_rate_pct,
            sharpe_ratio: result.performance_metrics.sharpe_ratio,
            max_drawdown: result.performance_metrics.max_drawdown_pct,
            total_fees: result.performance_metrics.total_fees_paid,
            markov_confidence: 0.0, // TODO: Add markov analysis to BacktestResult
            generated_at: Utc::now(),
        };
        
        let filename = format!("strategies/{}_strategy.json", pair.to_lowercase());
        let json = serde_json::to_string_pretty(&strategy)?;
        fs::write(&filename, json)?;
    }
    
    Ok(BacktestSummary {
        total_return: result.performance_metrics.total_return_pct,
        total_trades: result.performance_metrics.total_trades,
        win_rate: result.performance_metrics.win_rate_pct,
        sharpe_ratio: result.performance_metrics.sharpe_ratio,
        max_drawdown: result.performance_metrics.max_drawdown_pct,
    })
}

async fn generate_portfolio_report(results: &[(String, BacktestSummary)]) -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Generating portfolio analysis report...");
    
    // Create comprehensive report
    let mut report = String::new();
    report.push_str("# GBP Portfolio Backtesting Report\n\n");
    report.push_str(&format!("Generated: {}\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
    report.push_str(&format!("Total Pairs Analyzed: {}\n\n", results.len()));
    
    // Summary statistics
    let avg_return = results.iter().map(|(_, r)| r.total_return).sum::<f64>() / results.len() as f64;
    let avg_trades = results.iter().map(|(_, r)| r.total_trades).sum::<usize>() as f64 / results.len() as f64;
    let avg_sharpe = results.iter().map(|(_, r)| r.sharpe_ratio).sum::<f64>() / results.len() as f64;
    
    report.push_str("## Portfolio Summary\n\n");
    report.push_str(&format!("- Average Return: {:.2}%\n", avg_return * 100.0));
    report.push_str(&format!("- Average Trades: {:.1}\n", avg_trades));
    report.push_str(&format!("- Average Sharpe Ratio: {:.2}\n\n", avg_sharpe));
    
    // Individual pair results
    report.push_str("## Individual Pair Results\n\n");
    report.push_str("| Pair | Return | Trades | Win Rate | Sharpe | Max DD |\n");
    report.push_str("|------|--------|--------|----------|--------|---------|\n");
    
    for (pair, result) in results {
        report.push_str(&format!(
            "| {} | {:.2}% | {} | {:.1}% | {:.2} | {:.2}% |\n",
            pair,
            result.total_return * 100.0,
            result.total_trades,
            result.win_rate * 100.0,
            result.sharpe_ratio,
            result.max_drawdown * 100.0
        ));
    }
    
    // Save report
    fs::write("portfolio_analysis.md", report)?;
    info!("📊 Portfolio report saved: portfolio_analysis.md");
    
    Ok(())
}