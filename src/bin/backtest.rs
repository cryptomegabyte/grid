// Simple working backtest runner to demonstrate the system

use clap::{Parser, Subcommand};
use grid_trading_bot::{BacktestBuilder, OptimizationConfig, ParameterOptimizer};
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
    /// Optimize parameters for GBP pairs autonomously
    OptimizeGbp {
        /// Maximum number of pairs to optimize (default: all)
        #[arg(short, long)]
        limit: Option<usize>,
        /// Optimization strategy: grid-search, random-search, genetic-algorithm
        #[arg(short = 's', long, default_value = "random-search")]
        strategy: String,
        /// Number of iterations for optimization
        #[arg(short, long, default_value = "100")]
        iterations: usize,
        /// Include timeframe optimization
        #[arg(short, long)]
        timeframes: bool,
        /// Include risk management optimization
        #[arg(short = 'r', long)]
        risk_optimization: bool,
        /// Generate comprehensive optimization report
        #[arg(short = 'R', long)]
        report: bool,
    },
    /// Optimize parameters for a specific pair
    OptimizePair {
        /// Trading pair to optimize
        #[arg(short, long)]
        pair: String,
        /// Optimization strategy
        #[arg(short = 's', long, default_value = "random-search")]
        strategy: String,
        /// Number of iterations
        #[arg(short, long, default_value = "200")]
        iterations: usize,
        /// Include all optimization types
        #[arg(short, long)]
        comprehensive: bool,
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
        Commands::OptimizeGbp { limit, strategy, iterations, timeframes, risk_optimization, report } => {
            info!("🔧 Starting autonomous optimization for GBP pairs...");
            optimize_gbp_pairs(limit, &strategy, iterations, timeframes, risk_optimization, report).await?;
        }
        Commands::OptimizePair { pair, strategy, iterations, comprehensive } => {
            info!("⚙️ Optimizing parameters for {}...", pair);
            optimize_single_pair(&pair, &strategy, iterations, comprehensive).await?;
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

async fn optimize_gbp_pairs(
    limit: Option<usize>,
    strategy: &str,
    iterations: usize,
    include_timeframes: bool,
    include_risk: bool,
    generate_report: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use grid_trading_bot::clients::get_gbp_pair_names;
    
    info!("🔧 Starting autonomous parameter optimization...");
    
    // Get all GBP pairs
    let all_pairs = get_gbp_pair_names().await?;
    let pairs_to_optimize: Vec<String> = if let Some(limit) = limit {
        all_pairs.into_iter().take(limit).collect()
    } else {
        all_pairs
    };
    
    info!("🎯 Optimizing {} GBP pairs with {} strategy", pairs_to_optimize.len(), strategy);
    
    // Create optimization configuration
    let mut config = OptimizationConfig::default();
    
    // Configure strategy
    config.optimization_strategy = match strategy.to_lowercase().as_str() {
        "grid-search" => grid_trading_bot::optimization::OptimizationStrategy::GridSearch,
        "genetic-algorithm" => grid_trading_bot::optimization::OptimizationStrategy::GeneticAlgorithm {
            population: 50,
            generations: iterations / 50,
        },
        _ => grid_trading_bot::optimization::OptimizationStrategy::RandomSearch { iterations },
    };
    
    // Configure timeframes if requested
    if include_timeframes {
        config.timeframes = vec![5, 15, 30, 60, 240, 1440]; // 5m to 1d
    } else {
        config.timeframes = vec![60]; // Just 1h
    }
    
    // Configure risk optimization if requested
    if include_risk {
        config.risk_management.max_drawdown = vec![0.05, 0.10, 0.15, 0.20, 0.25];
        config.risk_management.stop_loss = vec![0.02, 0.05, 0.10, 0.15];
        config.risk_management.position_size = vec![0.05, 0.1, 0.25, 0.5];
    }
    
    let optimizer = ParameterOptimizer::new(config);
    let mut all_optimization_results = Vec::new();
    
    // Optimize each pair
    for (i, pair) in pairs_to_optimize.iter().enumerate() {
        info!("🔍 [{}/{}] Optimizing {}...", i + 1, pairs_to_optimize.len(), pair);
        
        match optimizer.optimize_pair(pair).await {
            Ok(results) => {
                if let Some(best_result) = results.first() {
                    info!("   🏆 Best score: {:.4} (levels={}, spacing={:.3}, timeframe={}m)", 
                          best_result.score,
                          best_result.parameters.grid_levels,
                          best_result.parameters.grid_spacing,
                          best_result.parameters.timeframe_minutes);
                    
                    // Save optimized strategy
                    let optimized_strategy = SimpleStrategy {
                        trading_pair: pair.clone(),
                        grid_levels: best_result.parameters.grid_levels,
                        grid_spacing: best_result.parameters.grid_spacing,
                        expected_return: best_result.backtest_result.total_return,
                        total_trades: best_result.backtest_result.total_trades,
                        win_rate: best_result.backtest_result.win_rate,
                        sharpe_ratio: best_result.backtest_result.sharpe_ratio,
                        max_drawdown: best_result.backtest_result.max_drawdown,
                        total_fees: 0.0, // TODO: Calculate from optimization
                        markov_confidence: 0.0, // TODO: Add from optimization
                        generated_at: Utc::now(),
                    };
                    
                    // Create optimized strategies directory
                    fs::create_dir_all("optimized_strategies")?;
                    let filename = format!("optimized_strategies/{}_optimized.json", pair.to_lowercase());
                    let json = serde_json::to_string_pretty(&optimized_strategy)?;
                    fs::write(&filename, json)?;
                }
                all_optimization_results.extend(results);
            }
            Err(e) => {
                warn!("   ❌ Failed to optimize {}: {}", pair, e);
            }
        }
        
        // Progress update
        if (i + 1) % 5 == 0 {
            info!("✅ Completed optimization for {}/{} pairs", i + 1, pairs_to_optimize.len());
        }
    }
    
    // Generate comprehensive report if requested
    if generate_report {
        generate_optimization_report(&all_optimization_results).await?;
    }
    
    info!("🎉 Autonomous optimization completed!");
    info!("📁 Optimized strategies saved in: optimized_strategies/");
    
    Ok(())
}

async fn optimize_single_pair(
    pair: &str,
    strategy: &str,
    iterations: usize,
    comprehensive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("⚙️ Starting comprehensive optimization for {}", pair);
    
    // Create comprehensive optimization configuration
    let mut config = OptimizationConfig::default();
    
    if comprehensive {
        // More extensive parameter ranges
        config.grid_levels.min = 3;
        config.grid_levels.max = 20;
        config.grid_spacing.min = 0.001;
        config.grid_spacing.max = 0.1;
        config.timeframes = vec![1, 5, 15, 30, 60, 240, 720, 1440]; // 1m to 1d
        
        // Extended date ranges for robust testing
        let now = Utc::now();
        config.date_ranges = vec![
            grid_trading_bot::optimization::DateRange {
                start: now - Duration::days(30),
                end: now,
                description: "Recent (30 days)".to_string(),
            },
            grid_trading_bot::optimization::DateRange {
                start: now - Duration::days(90),
                end: now - Duration::days(30),
                description: "Medium term (30-90 days ago)".to_string(),
            },
            grid_trading_bot::optimization::DateRange {
                start: now - Duration::days(180),
                end: now - Duration::days(90),
                description: "Long term (90-180 days ago)".to_string(),
            },
        ];
    }
    
    // Configure strategy
    config.optimization_strategy = match strategy.to_lowercase().as_str() {
        "grid-search" => grid_trading_bot::optimization::OptimizationStrategy::GridSearch,
        "genetic-algorithm" => grid_trading_bot::optimization::OptimizationStrategy::GeneticAlgorithm {
            population: 100,
            generations: iterations / 100,
        },
        _ => grid_trading_bot::optimization::OptimizationStrategy::RandomSearch { iterations },
    };
    
    let optimizer = ParameterOptimizer::new(config.clone());
    
    // Run optimization
    info!("📊 Running {} optimization with {} parameter combinations...", strategy, iterations);
    let results = optimizer.optimize_pair(pair).await?;
    
    if results.is_empty() {
        warn!("No optimization results generated for {}", pair);
        return Ok(());
    }
    
    // Display top results
    info!("🏆 Top 10 optimization results for {}:", pair);
    for (i, result) in results.iter().take(10).enumerate() {
        info!("   #{}: Score={:.4}, Levels={}, Spacing={:.3}, Timeframe={}m, Return={:.2}%",
              i + 1,
              result.score,
              result.parameters.grid_levels,
              result.parameters.grid_spacing,
              result.parameters.timeframe_minutes,
              result.backtest_result.total_return);
    }
    
    // Save best strategy
    let best_result = &results[0];
    let optimized_strategy = SimpleStrategy {
        trading_pair: pair.to_string(),
        grid_levels: best_result.parameters.grid_levels,
        grid_spacing: best_result.parameters.grid_spacing,
        expected_return: best_result.backtest_result.total_return,
        total_trades: best_result.backtest_result.total_trades,
        win_rate: best_result.backtest_result.win_rate,
        sharpe_ratio: best_result.backtest_result.sharpe_ratio,
        max_drawdown: best_result.backtest_result.max_drawdown,
        total_fees: 0.0,
        markov_confidence: 0.0,
        generated_at: Utc::now(),
    };
    
    fs::create_dir_all("optimized_strategies")?;
    let filename = format!("optimized_strategies/{}_comprehensive.json", pair.to_lowercase());
    let json = serde_json::to_string_pretty(&optimized_strategy)?;
    fs::write(&filename, json)?;
    
    // Generate detailed single-pair report
    generate_single_pair_optimization_report(pair, &results).await?;
    
    info!("✅ Optimization completed! Best strategy saved to: {}", filename);
    
    Ok(())
}

async fn generate_optimization_report(
    results: &[grid_trading_bot::optimization::OptimizationResult],
) -> Result<(), Box<dyn std::error::Error>> {
    if results.is_empty() {
        return Ok(());
    }
    
    let mut report = String::new();
    report.push_str("# GBP Pairs Autonomous Optimization Report\n\n");
    report.push_str(&format!("Generated: {}\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
    report.push_str(&format!("Total Optimization Runs: {}\n\n", results.len()));
    
    // Summary statistics
    let avg_score = results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64;
    let best_score = results.iter().map(|r| r.score).fold(f64::NEG_INFINITY, f64::max);
    let avg_return = results.iter().map(|r| r.backtest_result.total_return).sum::<f64>() / results.len() as f64;
    
    report.push_str("## Optimization Summary\n\n");
    report.push_str(&format!("- Average Optimization Score: {:.4}\n", avg_score));
    report.push_str(&format!("- Best Optimization Score: {:.4}\n", best_score));
    report.push_str(&format!("- Average Return: {:.2}%\n\n", avg_return));
    
    fs::write("optimization_report.md", report)?;
    info!("📊 Comprehensive optimization report saved: optimization_report.md");
    
    Ok(())
}

async fn generate_single_pair_optimization_report(
    pair: &str,
    results: &[grid_trading_bot::optimization::OptimizationResult],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut report = String::new();
    report.push_str(&format!("# {} Optimization Analysis\n\n", pair));
    report.push_str(&format!("Generated: {}\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
    report.push_str(&format!("Optimization Runs: {}\n\n", results.len()));
    
    if results.is_empty() {
        report.push_str("No optimization results available.\n");
    } else {
        let best = &results[0];
        
        report.push_str("## Best Configuration\n\n");
        report.push_str(&format!("- **Optimization Score**: {:.4}\n", best.score));
        report.push_str(&format!("- **Grid Levels**: {}\n", best.parameters.grid_levels));
        report.push_str(&format!("- **Grid Spacing**: {:.3}\n", best.parameters.grid_spacing));
        report.push_str(&format!("- **Timeframe**: {} minutes\n", best.parameters.timeframe_minutes));
        report.push_str(&format!("- **Total Return**: {:.2}%\n", best.backtest_result.total_return));
        report.push_str(&format!("- **Sharpe Ratio**: {:.2}\n", best.backtest_result.sharpe_ratio));
        report.push_str(&format!("- **Max Drawdown**: {:.2}%\n", best.backtest_result.max_drawdown));
        report.push_str(&format!("- **Total Trades**: {}\n", best.backtest_result.total_trades));
        report.push_str(&format!("- **Win Rate**: {:.1}%\n\n", best.backtest_result.win_rate));
    }
    
    let filename = format!("{}_optimization_analysis.md", pair.to_lowercase());
    fs::write(&filename, report)?;
    info!("📋 Detailed analysis saved: {}", filename);
    
    Ok(())
}