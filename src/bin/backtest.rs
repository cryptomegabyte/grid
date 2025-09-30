// Professional Grid Trading Backtest Runner
// Automatically discovers GBP pairs and generates optimized strategy configurations

use clap::{Parser, Subcommand};
use grid_trading_bot::{BacktestingEngine, KrakenHistoricalClient, BacktestBuilder, ParameterGrid, BacktestConfig};
use chrono::{Utc, Duration};
use serde::{Serialize, Deserialize};
use std::fs;
use tracing::{info, warn, error};

#[derive(Parser)]
#[command(name = "backtest")]
#[command(about = "Professional Grid Trading Backtest System")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Auto-discover and backtest all GBP pairs
    Auto {
        /// Days of historical data to analyze
        #[arg(short, long, default_value = "90")]
        days: i64,
        
        /// Starting capital in GBP
        #[arg(short, long, default_value = "10000")]
        capital: f64,
    },
    /// Backtest a specific pair
    Pair {
        /// Trading pair (e.g., XRPGBP, BTCGBP)
        pair: String,
        
        /// Days of historical data
        #[arg(short, long, default_value = "90")]
        days: i64,
        
        /// Starting capital in GBP
        #[arg(short, long, default_value = "10000")]
        capital: f64,
    },
    /// List available GBP pairs
    List,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OptimizedStrategy {
    pub trading_pair: String,
    pub timeframe: String,
    pub grid_levels: usize,
    pub grid_spacing: f64,
    pub initial_capital: f64,
    pub use_markov_predictions: bool,
    pub markov_lookback_periods: usize,
    
    // Performance metrics from backtest
    pub expected_annual_return: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub win_rate: f64,
    pub total_trades_tested: usize,
    
    // Risk management
    pub max_position_size_pct: f64,
    pub max_daily_loss_pct: f64,
    
    // Backtest metadata
    pub generated_at: chrono::DateTime<Utc>,
    pub backtest_period_days: i64,
    pub data_points_analyzed: usize,
    pub confidence_score: f64, // 0-100%
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Auto { days, capital } => {
            info!("🚀 Starting automatic GBP pairs discovery and backtesting");
            auto_backtest_gbp_pairs(days, capital).await?;
        }
        Commands::Pair { pair, days, capital } => {
            info!("🎯 Backtesting specific pair: {}", pair);
            backtest_single_pair(&pair, days, capital).await?;
        }
        Commands::List => {
            info!("📋 Discovering available GBP pairs");
            list_gbp_pairs().await?;
        }
    }
    
    Ok(())
}

async fn auto_backtest_gbp_pairs(days: i64, capital: f64) -> Result<(), Box<dyn std::error::Error>> {
    let gbp_pairs = discover_gbp_pairs().await?;
    
    info!("Found {} GBP trading pairs", gbp_pairs.len());
    
    // Ensure strategies directory exists
    fs::create_dir_all("strategies")?;
    
    let mut successful_strategies = Vec::new();
    
    for pair in gbp_pairs {
        info!("🔄 Backtesting {}", pair);
        
        match backtest_single_pair(&pair, days, capital).await {
            Ok(strategy) => {
                if strategy.confidence_score >= 60.0 && strategy.sharpe_ratio > 0.5 {
                    successful_strategies.push(strategy);
                    info!("✅ {} - Profitable strategy generated", pair);
                } else {
                    warn!("⚠️  {} - Strategy not profitable enough (confidence: {:.1}%, Sharpe: {:.2})", 
                          pair, strategy.confidence_score, strategy.sharpe_ratio);
                }
            }
            Err(e) => {
                error!("❌ {} - Backtest failed: {}", pair, e);
            }
        }
    }
    
    // Generate portfolio summary
    generate_portfolio_summary(&successful_strategies)?;
    
    info!("🎉 Backtest complete! Generated {} profitable strategies", successful_strategies.len());
    info!("📁 Strategy files saved in ./strategies/ directory");
    
    Ok(())
}

async fn backtest_single_pair(pair: &str, days: i64, capital: f64) -> Result<OptimizedStrategy, Box<dyn std::error::Error>> {
    let end_date = Utc::now();
    let start_date = end_date - Duration::days(days);
    
    info!("📊 Analyzing {} from {} to {}", pair, start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));
    
    // Parameter optimization to find best configuration
    let mut engine = BacktestBuilder::new()
        .with_initial_capital(capital)
        .build();
    
    let mut param_grid = ParameterGrid::new();
    let base_config = engine.config.clone();
    
    // Test different configurations
    param_grid.add_grid_spacing_sweep(&base_config, &[0.005, 0.01, 0.015, 0.02, 0.025, 0.03]);
    param_grid.add_grid_levels_sweep(&base_config, &[3, 5, 7, 10]);
    
    info!("🔧 Optimizing parameters across {} configurations", param_grid.configurations.len());
    
    let optimization_results = engine.optimize_parameters(
        pair,
        start_date,
        end_date,
        60, // 1-hour timeframe
        param_grid,
    ).await?;
    
    // Select best configuration
    let best_result = optimization_results.iter()
        .max_by(|a, b| a.sharpe_ratio.partial_cmp(&b.sharpe_ratio).unwrap_or(std::cmp::Ordering::Equal))
        .ok_or("No valid backtest results")?;
    
    // Calculate confidence score based on multiple factors
    let confidence_score = calculate_confidence_score(best_result, &optimization_results);
    
    let strategy = OptimizedStrategy {
        trading_pair: pair.to_string(),
        timeframe: "1h".to_string(),
        grid_levels: best_result.config.grid_levels,
        grid_spacing: best_result.config.base_grid_spacing,
        initial_capital: capital,
        use_markov_predictions: best_result.config.use_markov_predictions,
        markov_lookback_periods: best_result.config.markov_lookback_periods,
        
        expected_annual_return: best_result.annualized_return,
        sharpe_ratio: best_result.sharpe_ratio,
        max_drawdown: best_result.max_drawdown,
        win_rate: best_result.win_rate,
        total_trades_tested: best_result.num_trades,
        
        max_position_size_pct: best_result.config.risk_config.max_position_size_pct,
        max_daily_loss_pct: best_result.config.risk_config.max_daily_loss_pct,
        
        generated_at: Utc::now(),
        backtest_period_days: days,
        data_points_analyzed: best_result.data_points,
        confidence_score,
    };
    
    // Save strategy to file
    let filename = format!("strategies/{}_optimized.json", pair.to_lowercase());
    let json = serde_json::to_string_pretty(&strategy)?;
    fs::write(&filename, json)?;
    
    info!("💾 Strategy saved: {}", filename);
    info!("📈 Expected Return: {:.2}% | Sharpe: {:.2} | Confidence: {:.1}%", 
          strategy.expected_annual_return, strategy.sharpe_ratio, strategy.confidence_score);
    
    Ok(strategy)
}

async fn discover_gbp_pairs() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    info!("🔍 Discovering GBP trading pairs from Kraken...");
    
    let client = KrakenHistoricalClient::new();
    
    // Common crypto assets that usually have GBP pairs
    let potential_pairs = vec![
        "XRPGBP", "BTCGBP", "ETHGBP", "ADAGBP", "DOTGBP", 
        "LINKGBP", "LTCGBP", "BCHGBP", "XLMGBP", "EOSLTCGBP"
    ];
    
    let mut valid_pairs = Vec::new();
    
    for pair in potential_pairs {
        // Test if pair exists by trying to fetch recent data
        let test_date = Utc::now() - Duration::days(7);
        match client.fetch_ohlc(pair, 240, Some(test_date)).await {
            Ok(data) => {
                if data.len() > 10 { // Ensure sufficient data
                    valid_pairs.push(pair.to_string());
                    info!("✅ {} - Valid pair with {} data points", pair, data.len());
                } else {
                    warn!("⚠️  {} - Insufficient data", pair);
                }
            }
            Err(_) => {
                warn!("❌ {} - Pair not available", pair);
            }
        }
    }
    
    Ok(valid_pairs)
}

async fn list_gbp_pairs() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = discover_gbp_pairs().await?;
    
    println!("📋 Available GBP Trading Pairs:");
    println!("================================");
    
    for (i, pair) in pairs.iter().enumerate() {
        println!("{}. {}", i + 1, pair);
    }
    
    println!("\nTo backtest all pairs: cargo run --bin backtest auto");
    println!("To backtest specific pair: cargo run --bin backtest pair XRPGBP");
    
    Ok(())
}

fn calculate_confidence_score(best_result: &OptimizationResult, all_results: &[OptimizationResult]) -> f64 {
    let mut score = 50.0; // Base score
    
    // Factor 1: Sharpe ratio quality
    if best_result.sharpe_ratio > 2.0 { score += 20.0; }
    else if best_result.sharpe_ratio > 1.0 { score += 10.0; }
    else if best_result.sharpe_ratio > 0.5 { score += 5.0; }
    
    // Factor 2: Number of trades (more = better statistical significance)
    if best_result.num_trades > 100 { score += 15.0; }
    else if best_result.num_trades > 50 { score += 10.0; }
    else if best_result.num_trades > 20 { score += 5.0; }
    
    // Factor 3: Consistency across different parameters
    let positive_results = all_results.iter().filter(|r| r.total_return > 0.0).count();
    let consistency_pct = positive_results as f64 / all_results.len() as f64;
    score += consistency_pct * 15.0;
    
    // Factor 4: Drawdown management
    if best_result.max_drawdown < 0.05 { score += 10.0; }
    else if best_result.max_drawdown < 0.10 { score += 5.0; }
    
    score.min(100.0)
}

fn generate_portfolio_summary(strategies: &[OptimizedStrategy]) -> Result<(), Box<dyn std::error::Error>> {
    let summary = serde_json::json!({
        "generated_at": Utc::now(),
        "total_strategies": strategies.len(),
        "strategies": strategies,
        "portfolio_metrics": {
            "avg_expected_return": strategies.iter().map(|s| s.expected_annual_return).sum::<f64>() / strategies.len() as f64,
            "avg_sharpe_ratio": strategies.iter().map(|s| s.sharpe_ratio).sum::<f64>() / strategies.len() as f64,
            "avg_confidence": strategies.iter().map(|s| s.confidence_score).sum::<f64>() / strategies.len() as f64,
        }
    });
    
    fs::write("strategies/portfolio_summary.json", serde_json::to_string_pretty(&summary)?)?;
    info!("📋 Portfolio summary saved: strategies/portfolio_summary.json");
    
    Ok(())
}

// Placeholder for OptimizationResult - this should match your actual type
#[derive(Clone)]
struct OptimizationResult {
    config: BacktestConfig,
    total_return: f64,
    annualized_return: f64,
    sharpe_ratio: f64,
    max_drawdown: f64,
    win_rate: f64,
    num_trades: usize,
    data_points: usize,
}