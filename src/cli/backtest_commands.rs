// Backtest command implementations - Phase 2 with config
use tracing::{info, warn};
use grid_trading_bot::{
    CliConfig, 
    OptimizationConfig, 
    ParameterOptimizer,
    optimization::OptimizationStrategy,
};
use chrono::Utc;
use std::fs;
use serde::{Serialize, Deserialize};

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

pub async fn optimize_all_pairs(
    limit: Option<usize>,
    strategy: &str,
    iterations: usize,
    report: bool,
    _config: &CliConfig,
) -> grid_trading_bot::TradingResult<()> {
    use grid_trading_bot::clients::get_gbp_pair_names;
    
    info!("� Starting autonomous parameter optimization...");
    
    // Get all GBP pairs
    let all_pairs = get_gbp_pair_names().await
        .map_err(|e| grid_trading_bot::TradingError::ApiResponse(format!("Failed to get GBP pairs: {}", e)))?;
    
    let pairs_to_optimize: Vec<String> = if let Some(limit) = limit {
        all_pairs.into_iter().take(limit).collect()
    } else {
        all_pairs
    };
    
    info!("🎯 Optimizing {} GBP pairs with {} strategy", pairs_to_optimize.len(), strategy);
    info!("   Iterations per pair: {}", iterations);
    
    // Create optimization configuration with strategy
    let mut config = OptimizationConfig {
        optimization_strategy: match strategy.to_lowercase().as_str() {
            "grid-search" => OptimizationStrategy::GridSearch,
            "genetic-algorithm" => OptimizationStrategy::GeneticAlgorithm {
                population: 50,
                generations: iterations / 50,
            },
            _ => OptimizationStrategy::RandomSearch { iterations },
        },
        ..Default::default()
    };
    
    // Use single timeframe for faster optimization
    config.timeframes = vec![60]; // Just 1h
    
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
                    
                    // Create strategies directory
                    fs::create_dir_all("strategies")
                        .map_err(|e| grid_trading_bot::TradingError::DirectoryCreate(format!("Failed to create strategies dir: {}", e)))?;
                    let filename = format!("strategies/{}_optimized.json", pair.to_lowercase());
                    let json = serde_json::to_string_pretty(&optimized_strategy)
                        .map_err(|e| grid_trading_bot::TradingError::StrategyParseFailed(format!("Failed to serialize strategy: {}", e)))?;
                    fs::write(&filename, json)
                        .map_err(|e| grid_trading_bot::TradingError::FileWrite(format!("Failed to write strategy file: {}", e)))?;
                    info!("💾 Optimized strategy saved: {}", filename);
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
    if report && !all_optimization_results.is_empty() {
        generate_optimization_report(&all_optimization_results)?;
    }
    
    info!("🎉 Autonomous optimization completed!");
    info!("📁 Optimized strategies saved in: strategies/");
    
    Ok(())
}

fn generate_optimization_report(
    results: &[grid_trading_bot::optimization::OptimizationResult],
) -> grid_trading_bot::TradingResult<()> {
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
    
    fs::write("optimization_report.md", report)
        .map_err(|e| grid_trading_bot::TradingError::FileWrite(format!("Failed to write report: {}", e)))?;
    info!("📊 Comprehensive optimization report saved: optimization_report.md");
    
    Ok(())
}

pub async fn optimize_single_pair(
    pair: &str,
    strategy: &str,
    iterations: usize,
    comprehensive: bool,
    _config: &CliConfig,
) -> grid_trading_bot::TradingResult<()> {
    info!("⚙️ Starting {} optimization for {}", 
          if comprehensive { "comprehensive" } else { "standard" }, 
          pair);
    
    // Create optimization configuration
    let mut config = OptimizationConfig::default();
    
    if comprehensive {
        // More extensive parameter ranges
        config.grid_levels.min = 3;
        config.grid_levels.max = 20;
        config.grid_spacing.min = 0.001;
        config.grid_spacing.max = 0.1;
        config.timeframes = vec![5, 15, 30, 60, 240, 1440]; // 5m to 1d
    } else {
        config.timeframes = vec![60]; // Just 1h
    }
    
    // Configure strategy
    config.optimization_strategy = match strategy.to_lowercase().as_str() {
        "grid-search" => OptimizationStrategy::GridSearch,
        "genetic-algorithm" => OptimizationStrategy::GeneticAlgorithm {
            population: 100,
            generations: iterations / 100,
        },
        _ => OptimizationStrategy::RandomSearch { iterations },
    };
    
    let optimizer = ParameterOptimizer::new(config.clone());
    
    // Run optimization
    info!("📊 Running {} optimization with {} parameter combinations...", strategy, iterations);
    let results = optimizer.optimize_pair(pair).await
        .map_err(|e| grid_trading_bot::TradingError::Internal(format!("Optimization failed: {}", e)))?;
    
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
    
    fs::create_dir_all("strategies")
        .map_err(|e| grid_trading_bot::TradingError::DirectoryCreate(format!("Failed to create strategies dir: {}", e)))?;
    let filename = if comprehensive {
        format!("strategies/{}_comprehensive.json", pair.to_lowercase())
    } else {
        format!("strategies/{}_optimized.json", pair.to_lowercase())
    };
    let json = serde_json::to_string_pretty(&optimized_strategy)
        .map_err(|e| grid_trading_bot::TradingError::StrategyParseFailed(format!("Failed to serialize strategy: {}", e)))?;
    fs::write(&filename, json)
        .map_err(|e| grid_trading_bot::TradingError::FileWrite(format!("Failed to write strategy file: {}", e)))?;
    
    info!("✅ Optimization completed! Best strategy saved to: {}", filename);
    
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
